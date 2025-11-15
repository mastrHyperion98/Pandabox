# Test Installer Script
# Quickly test the built installer in a safe way

param(
    [string]$MsiPath = "",
    [switch]$Install = $false,
    [switch]$Uninstall = $false,
    [switch]$Info = $false
)

$ErrorActionPreference = "Stop"

function Write-ColorOutput($ForegroundColor, $Message) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    Write-Output $Message
    $host.UI.RawUI.ForegroundColor = $fc
}

# Find MSI if not specified
if ([string]::IsNullOrEmpty($MsiPath)) {
    $OutputDir = Join-Path $PSScriptRoot "output"
    $MsiFiles = Get-ChildItem -Path $OutputDir -Filter "*.msi" -ErrorAction SilentlyContinue
    
    if ($MsiFiles.Count -eq 0) {
        Write-ColorOutput Red "No MSI files found in output directory."
        Write-Output "Run build-installer.ps1 first."
        exit 1
    }
    
    # Use the most recent MSI
    $MsiPath = ($MsiFiles | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName
    Write-ColorOutput Cyan "Using MSI: $MsiPath"
}

if (-not (Test-Path $MsiPath)) {
    Write-ColorOutput Red "MSI file not found: $MsiPath"
    exit 1
}

# Show MSI information
if ($Info -or (-not $Install -and -not $Uninstall)) {
    Write-ColorOutput Yellow "`n=== MSI Information ==="
    
    $MsiFile = Get-Item $MsiPath
    Write-Output "File: $($MsiFile.Name)"
    Write-Output "Size: $([math]::Round($MsiFile.Length / 1MB, 2)) MB"
    Write-Output "Modified: $($MsiFile.LastWriteTime)"
    
    # Try to extract product info using Windows Installer COM object
    try {
        $installer = New-Object -ComObject WindowsInstaller.Installer
        $database = $installer.GetType().InvokeMember("OpenDatabase", "InvokeMethod", $null, $installer, @($MsiPath, 0))
        
        $view = $database.GetType().InvokeMember("OpenView", "InvokeMethod", $null, $database, @("SELECT * FROM Property"))
        $view.GetType().InvokeMember("Execute", "InvokeMethod", $null, $view, $null)
        
        $properties = @{}
        while ($true) {
            $record = $view.GetType().InvokeMember("Fetch", "InvokeMethod", $null, $view, $null)
            if ($null -eq $record) { break }
            
            $prop = $record.GetType().InvokeMember("StringData", "GetProperty", $null, $record, 1)
            $value = $record.GetType().InvokeMember("StringData", "GetProperty", $null, $record, 2)
            $properties[$prop] = $value
        }
        
        Write-Output "`nProduct Information:"
        Write-Output "  Name: $($properties['ProductName'])"
        Write-Output "  Version: $($properties['ProductVersion'])"
        Write-Output "  Manufacturer: $($properties['Manufacturer'])"
        Write-Output "  Product Code: $($properties['ProductCode'])"
        
        [System.Runtime.Interopservices.Marshal]::ReleaseComObject($database) | Out-Null
        [System.Runtime.Interopservices.Marshal]::ReleaseComObject($installer) | Out-Null
    }
    catch {
        Write-ColorOutput Yellow "Could not extract detailed MSI information."
    }
    
    Write-Output ""
}

# Install
if ($Install) {
    Write-ColorOutput Yellow "`n=== Installing Pandabox ==="
    Write-Output "This will install Pandabox on your system."
    Write-Output "You may be prompted for administrator privileges."
    Write-Output ""
    
    $confirm = Read-Host "Continue? (y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Output "Installation cancelled."
        exit 0
    }
    
    Write-ColorOutput Cyan "Installing..."
    $logPath = Join-Path $PSScriptRoot "install.log"
    
    Start-Process msiexec.exe -ArgumentList "/i `"$MsiPath`" /l*v `"$logPath`"" -Wait -NoNewWindow
    
    if ($LASTEXITCODE -eq 0) {
        Write-ColorOutput Green "`n✓ Installation completed successfully!"
        Write-Output "`nVerifying installation..."
        
        $exePath = "C:\Program Files\Pandabox\bin\Pandabox.exe"
        if (Test-Path $exePath) {
            Write-ColorOutput Green "✓ Binary found: $exePath"
        }
        
        $startMenuPath = "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Pandabox\Pandabox.lnk"
        if (Test-Path $startMenuPath) {
            Write-ColorOutput Green "✓ Start Menu shortcut created"
        }
        
        $desktopPath = "$env:USERPROFILE\Desktop\Pandabox.lnk"
        if (Test-Path $desktopPath) {
            Write-ColorOutput Green "✓ Desktop shortcut created"
        }
        
        Write-Output "`nYou can now launch Pandabox from:"
        Write-Output "  - Desktop shortcut"
        Write-Output "  - Start Menu"
        Write-Output "  - Or run: & '$exePath'"
    }
    else {
        Write-ColorOutput Red "`n✗ Installation failed with exit code: $LASTEXITCODE"
        Write-Output "Check log file: $logPath"
    }
}

# Uninstall
if ($Uninstall) {
    Write-ColorOutput Yellow "`n=== Uninstalling Pandabox ==="
    Write-Output "This will remove Pandabox from your system."
    Write-Output ""
    
    $confirm = Read-Host "Continue? (y/N)"
    if ($confirm -ne "y" -and $confirm -ne "Y") {
        Write-Output "Uninstallation cancelled."
        exit 0
    }
    
    Write-ColorOutput Cyan "Uninstalling..."
    $logPath = Join-Path $PSScriptRoot "uninstall.log"
    
    Start-Process msiexec.exe -ArgumentList "/x `"$MsiPath`" /l*v `"$logPath`"" -Wait -NoNewWindow
    
    if ($LASTEXITCODE -eq 0) {
        Write-ColorOutput Green "`n✓ Uninstallation completed successfully!"
        
        Write-Output "`nNote: User data is preserved at:"
        Write-Output "  $env:LOCALAPPDATA\pandabox\"
        Write-Output ""
        Write-Output "To completely remove all data, delete that directory manually."
    }
    else {
        Write-ColorOutput Red "`n✗ Uninstallation failed with exit code: $LASTEXITCODE"
        Write-Output "Check log file: $logPath"
    }
}

# Show usage if no action specified
if (-not $Install -and -not $Uninstall -and -not $Info) {
    Write-Output ""
    Write-ColorOutput Cyan "Usage:"
    Write-Output "  .\test-installer.ps1 -Info              # Show MSI information"
    Write-Output "  .\test-installer.ps1 -Install           # Install Pandabox"
    Write-Output "  .\test-installer.ps1 -Uninstall         # Uninstall Pandabox"
    Write-Output "  .\test-installer.ps1 -MsiPath <path>    # Specify MSI file"
    Write-Output ""
}
