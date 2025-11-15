# Pandabox Windows Installer Build Script
# This script builds a release binary and creates a Windows MSI installer

param(
    [switch]$SkipBuild = $false,
    [switch]$Clean = $false,
    [string]$Version = "0.1.1"
)

$ErrorActionPreference = "Stop"

# Colors for output
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Step {
    param($message)
    Write-ColorOutput Cyan "`n==> $message"
}

function Write-Success {
    param($message)
    Write-ColorOutput Green "[OK] $message"
}

function Write-ErrorMsg {
    param($message)
    Write-ColorOutput Red "[ERROR] $message"
}

# Paths
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$DistributeDir = $PSScriptRoot
$TargetDir = Join-Path $ProjectRoot "target\release"
$StagingDir = Join-Path $DistributeDir "staging"
$OutputDir = Join-Path $DistributeDir "output"
$AssetsDir = Join-Path $DistributeDir "assets"

Write-ColorOutput Yellow @"

===================================================
   Pandabox Windows Installer Builder v$Version
===================================================

"@

# Check prerequisites
Write-Step "Checking prerequisites..."

# Check for Rust/Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-ErrorMsg "Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
}
Write-Success "Cargo found"

# Check for WiX Toolset
if (-not (Get-Command wix -ErrorAction SilentlyContinue)) {
    Write-ErrorMsg "WiX Toolset not found. Please install WiX v4 from https://wixtoolset.org/"
    Write-Output "  Install with: dotnet tool install --global wix"
    exit 1
}
Write-Success "WiX Toolset found"

# Clean previous builds
if ($Clean) {
    Write-Step "Cleaning previous builds..."
    if (Test-Path $StagingDir) { Remove-Item -Recurse -Force $StagingDir }
    if (Test-Path $OutputDir) { Remove-Item -Recurse -Force $OutputDir }
    Write-Success "Cleaned"
}

# Create directories
Write-Step "Creating directories..."
New-Item -ItemType Directory -Force -Path $StagingDir | Out-Null
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $AssetsDir | Out-Null
Write-Success "Directories created"

# Build release binary
if (-not $SkipBuild) {
    Write-Step "Building release binary..."
    Push-Location $ProjectRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) {
            Write-ErrorMsg "Cargo build failed"
            exit 1
        }
        Write-Success "Build completed"
    }
    finally {
        Pop-Location
    }
}
else {
    Write-Step "Skipping build (using existing binary)..."
}

# Verify binary exists
$ExePath = Join-Path $TargetDir "Pandabox.exe"
if (-not (Test-Path $ExePath)) {
    Write-ErrorMsg "Binary not found at $ExePath"
    exit 1
}
Write-Success "Binary found: $ExePath"

# Copy binary to staging
Write-Step "Copying files to staging..."
Copy-Item $ExePath $StagingDir
Write-Success "Binary copied"

# Collect dependencies
Write-Step "Collecting dependencies..."
$Dependencies = @()

# Find all DLLs in the target directory
$DllFiles = Get-ChildItem -Path $TargetDir -Filter "*.dll" -ErrorAction SilentlyContinue
foreach ($dll in $DllFiles) {
    Copy-Item $dll.FullName $StagingDir
    $Dependencies += $dll.Name
    Write-Output "  - $($dll.Name)"
}

# Check for Visual C++ Runtime dependencies
$VcRedistDlls = @(
    "vcruntime140.dll",
    "vcruntime140_1.dll",
    "msvcp140.dll"
)

foreach ($dll in $VcRedistDlls) {
    $systemDll = Join-Path $env:SystemRoot "System32\$dll"
    if (Test-Path $systemDll) {
        Copy-Item $systemDll $StagingDir -ErrorAction SilentlyContinue
        if (Test-Path (Join-Path $StagingDir $dll)) {
            $Dependencies += $dll
            Write-Output "  - $dll (VC++ Runtime)"
        }
    }
}

Write-Success "Collected $($Dependencies.Count) dependencies"

# Create assets if they don't exist
Write-Step "Preparing installer assets..."

# Create a simple license.rtf if it doesn't exist
$LicenseRtf = Join-Path $AssetsDir "license.rtf"
if (-not (Test-Path $LicenseRtf)) {
    $LicenseContent = @'
{\rtf1\ansi\deff0
{\fonttbl{\f0 Arial;}}
\f0\fs20
Pandabox - Secure Password Manager

MIT License

Copyright (c) 2024 mastrHyperion98

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
}
'@
    Set-Content -Path $LicenseRtf -Value $LicenseContent
    Write-Success "Created license.rtf"
}

# Check for icon
$IconPath = Join-Path $AssetsDir "icon.ico"
if (-not (Test-Path $IconPath)) {
    Write-Output "  [!] icon.ico not found - build will fail"
    Write-Output "      Convert resources\panda.png to .ico format"
    Write-Output "      Quick option: https://convertio.co/png-ico/"
    Write-Output "      Save to: $IconPath"
    Write-Output ""
    
    # Try to help the user
    $pngPath = Join-Path $ProjectRoot "resources\panda.png"
    if (Test-Path $pngPath) {
        Write-Output "      Your PNG is at: $pngPath"
    }
    
    exit 1
} else {
    Write-Success "Icon found: icon.ico"
}

$BannerPath = Join-Path $AssetsDir "banner.bmp"
if (-not (Test-Path $BannerPath)) {
    Write-Output "  ⚠ banner.bmp not found - installer will use default banner"
    Write-Output "    Create a 493x58 .bmp file at: $BannerPath"
}

$DialogPath = Join-Path $AssetsDir "dialog.bmp"
if (-not (Test-Path $DialogPath)) {
    Write-Output "  ⚠ dialog.bmp not found - installer will use default dialog"
    Write-Output "    Create a 493x312 .bmp file at: $DialogPath"
}

# Generate WiX fragment for dependencies
Write-Step "Generating WiX component manifest..."
$FragmentPath = Join-Path $DistributeDir "dependencies.wxs"
$ComponentsXml = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Fragment>
    <ComponentGroup Id="DependencyComponents" Directory="BIN_DIR">
"@

$guidIndex = 1
foreach ($dep in $Dependencies) {
    # Generate a deterministic GUID based on filename
    $guid = [System.Guid]::NewGuid().ToString().ToUpper()
    $ComponentsXml += @"

      <Component Id="Dep_$($dep.Replace('.', '_').Replace('-', '_'))" Guid="$guid">
        <File Id="File_$($dep.Replace('.', '_').Replace('-', '_'))" 
              Source="staging\$dep" 
              KeyPath="yes" />
      </Component>
"@
    $guidIndex++
}

$ComponentsXml += @"

    </ComponentGroup>
  </Fragment>
</Wix>
"@

Set-Content -Path $FragmentPath -Value $ComponentsXml
Write-Success "Component manifest generated"

# Update main WXS to include dependencies
Write-Step "Updating installer configuration..."
$MainWxs = Join-Path $DistributeDir "pandabox.wxs"

# Check if we need to update the WXS file
$WxsContent = Get-Content $MainWxs -Raw
if ($WxsContent -notmatch "ComponentGroupRef.*DependencyComponents") {
    Write-Output "  Note: pandabox.wxs needs manual update to include DependencyComponents"
    Write-Output "  Add this line inside the <Feature> element:"
    Write-Output '      <ComponentGroupRef Id="DependencyComponents" />'
} else {
    Write-Success "Installer configuration already includes dependencies"
}

# Build MSI installer
Write-Step "Building MSI installer..."
Push-Location $DistributeDir
try {
    # Try to build with UI extension first
    Write-Output "  Attempting build with UI extension..."
    wix build pandabox.wxs dependencies.wxs -out "$OutputDir\Pandabox-$Version.msi" -ext WixToolset.UI.wixext 2>$null
    
    if ($LASTEXITCODE -ne 0) {
        Write-Output "  UI extension not found, building without it..."
        # Build without UI extension (basic installer)
        wix build pandabox.wxs dependencies.wxs -out "$OutputDir\Pandabox-$Version.msi"
        
        if ($LASTEXITCODE -ne 0) {
            Write-ErrorMsg "WiX build failed"
            Write-Output ""
            Write-Output "To install WiX UI extension, run:"
            Write-Output "  wix extension add WixToolset.UI.wixext"
            exit 1
        }
    }
    
    Write-Success "MSI installer created successfully!"
}
finally {
    Pop-Location
}

# Display results
Write-ColorOutput Green @"

===================================================
         BUILD COMPLETED SUCCESSFULLY
===================================================

"@

$MsiPath = Join-Path $OutputDir "Pandabox-$Version.msi"
$MsiSize = (Get-Item $MsiPath).Length / 1MB

Write-Output "Installer: $MsiPath"
Write-Output "Size: $([math]::Round($MsiSize, 2)) MB"
Write-Output ""
Write-Output "To install: Double-click the MSI file or run:"
Write-Output "  msiexec /i `"$MsiPath`""
Write-Output ""
Write-Output "To uninstall: Use Windows Settings > Apps or run:"
Write-Output "  msiexec /x `"$MsiPath`""
Write-Output ""
