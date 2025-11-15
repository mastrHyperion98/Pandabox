# Dependency Collection Script for Pandabox
# Analyzes the binary and collects all required DLLs

param(
    [string]$BinaryPath = "..\target\release\Pandabox.exe",
    [string]$OutputDir = "staging"
)

$ErrorActionPreference = "Stop"

Write-Host "Collecting dependencies for: $BinaryPath" -ForegroundColor Cyan

if (-not (Test-Path $BinaryPath)) {
    Write-Host "Error: Binary not found at $BinaryPath" -ForegroundColor Red
    exit 1
}

# Create output directory
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Copy the main binary
Copy-Item $BinaryPath $OutputDir -Force
Write-Host "✓ Copied main binary" -ForegroundColor Green

# Get the directory of the binary
$BinaryDir = Split-Path -Parent $BinaryPath

# Function to find DLL dependencies using dumpbin (if available)
function Get-DllDependencies {
    param([string]$FilePath)
    
    $dependencies = @()
    
    # Try using dumpbin (Visual Studio tool)
    if (Get-Command dumpbin -ErrorAction SilentlyContinue) {
        $dumpbinOutput = dumpbin /dependents $FilePath 2>$null
        foreach ($line in $dumpbinOutput) {
            if ($line -match '^\s+(\S+\.dll)\s*$') {
                $dependencies += $matches[1]
            }
        }
    }
    
    return $dependencies
}

# Collect DLLs from the binary directory
Write-Host "`nSearching for DLLs in binary directory..." -ForegroundColor Cyan
$DllsFound = Get-ChildItem -Path $BinaryDir -Filter "*.dll" -ErrorAction SilentlyContinue

foreach ($dll in $DllsFound) {
    Copy-Item $dll.FullName $OutputDir -Force
    Write-Host "  + $($dll.Name)" -ForegroundColor Gray
}

# Check for common runtime dependencies
Write-Host "`nChecking for runtime dependencies..." -ForegroundColor Cyan

$RuntimeDlls = @{
    "Visual C++ Runtime" = @(
        "vcruntime140.dll",
        "vcruntime140_1.dll", 
        "msvcp140.dll",
        "concrt140.dll"
    )
    "Universal CRT" = @(
        "ucrtbase.dll"
    )
}

$SystemDirs = @(
    "$env:SystemRoot\System32",
    "$env:SystemRoot\SysWOW64"
)

foreach ($category in $RuntimeDlls.Keys) {
    Write-Host "`n  $category" -ForegroundColor Yellow
    foreach ($dll in $RuntimeDlls[$category]) {
        $found = $false
        foreach ($dir in $SystemDirs) {
            $dllPath = Join-Path $dir $dll
            if (Test-Path $dllPath) {
                try {
                    Copy-Item $dllPath $OutputDir -Force -ErrorAction SilentlyContinue
                    if (Test-Path (Join-Path $OutputDir $dll)) {
                        Write-Host "    ✓ $dll" -ForegroundColor Green
                        $found = $true
                        break
                    }
                } catch {
                    # Some system DLLs can't be copied, which is fine
                }
            }
        }
        if (-not $found) {
            Write-Host "    - $dll (not found or not needed)" -ForegroundColor DarkGray
        }
    }
}

# Summary
Write-Host "`n" + ("="*60) -ForegroundColor Cyan
$CollectedFiles = Get-ChildItem -Path $OutputDir
Write-Host "Total files collected: $($CollectedFiles.Count)" -ForegroundColor Green
$TotalSize = ($CollectedFiles | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host "Total size: $([math]::Round($TotalSize, 2)) MB" -ForegroundColor Green
Write-Host ("="*60) -ForegroundColor Cyan

Write-Host "`nFiles ready in: $OutputDir" -ForegroundColor Cyan
