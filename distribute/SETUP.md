# Windows Installer Setup Guide

## First-Time Setup

### 1. Install Prerequisites

#### WiX Toolset v4 (Required)

```powershell
# Install via .NET CLI
dotnet tool install --global wix

# Verify installation
wix --version
```

If `dotnet` is not found, install [.NET SDK](https://dotnet.microsoft.com/download) first.

#### Alternative: Manual Installation

Download from [WiX Toolset website](https://wixtoolset.org/docs/intro/#getting-started)

### 2. Verify Rust Installation

```powershell
cargo --version
rustc --version
```

If not installed, get Rust from [rustup.rs](https://rustup.rs/)

### 3. Add Branding Assets (Optional)

Create these files in `distribute/assets/`:

- **icon.ico** - 256x256 application icon
- **banner.bmp** - 493x58 installer banner
- **dialog.bmp** - 493x312 installer background

The build will work without these, but they make the installer look professional.

## Building Your First Installer

### Option 1: Full Build (Recommended)

```powershell
cd distribute
.\build-installer.ps1
```

This will:
1. Compile Pandabox in release mode
2. Collect all dependencies
3. Generate WiX components
4. Build the MSI installer

**Time:** ~2-5 minutes (depending on your machine)

### Option 2: Quick Build (Binary Already Compiled)

```powershell
cd distribute
.\build-installer.ps1 -SkipBuild
```

Use this if you've already run `cargo build --release` manually.

### Option 3: Clean Build

```powershell
cd distribute
.\build-installer.ps1 -Clean
```

Removes all previous build artifacts before building.

## Testing the Installer

### Install

1. Navigate to `distribute/output/`
2. Double-click `Pandabox-0.1.1.msi`
3. Follow the installation wizard
4. Check for:
   - Desktop shortcut
   - Start Menu entry
   - Application launches correctly

### Verify Installation

```powershell
# Check installation directory
Test-Path "C:\Program Files\Pandabox\bin\Pandabox.exe"

# Check Start Menu
Test-Path "$env:APPDATA\Microsoft\Windows\Start Menu\Programs\Pandabox\Pandabox.lnk"

# Check Desktop shortcut
Test-Path "$env:USERPROFILE\Desktop\Pandabox.lnk"
```

### Uninstall

**Method 1: Windows Settings**
1. Open Settings → Apps → Installed apps
2. Find "Pandabox"
3. Click Uninstall

**Method 2: Command Line**
```powershell
msiexec /x "distribute\output\Pandabox-0.1.1.msi"
```

## Common Issues

### Issue: "wix: command not found"

**Solution:**
```powershell
# Add .NET tools to PATH
$env:PATH += ";$env:USERPROFILE\.dotnet\tools"

# Make permanent
[Environment]::SetEnvironmentVariable(
    "Path",
    [Environment]::GetEnvironmentVariable("Path", "User") + ";$env:USERPROFILE\.dotnet\tools",
    "User"
)
```

### Issue: "Execution policy" error

**Solution:**
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Issue: Missing DLLs when running installed app

**Solution:**

1. Install [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)

2. Or build with static linking:
   ```toml
   # Add to Cargo.toml
   [profile.release]
   rustflags = ["-C", "target-feature=+crt-static"]
   ```

### Issue: SmartScreen warning

**Solution:**

This is normal for unsigned applications. Users can click "More info" → "Run anyway"

For production, consider [code signing](https://docs.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools):
- Get a code signing certificate
- Sign the MSI: `signtool sign /f cert.pfx /p password Pandabox.msi`

## Advanced Configuration

### Change Installation Directory

Edit `pandabox.wxs`, line 18:
```xml
<Directory Id="INSTALLFOLDER" Name="Pandabox">
```

### Add File Associations

Add to `pandabox.wxs` inside `<Component>`:
```xml
<ProgId Id="Pandabox.Document" Description="Pandabox Document">
  <Extension Id="pbox" ContentType="application/x-pandabox">
    <Verb Id="open" Command="Open" TargetFile="PandaboxExeFile" />
  </Extension>
</ProgId>
```

### Per-User Installation

Change `<Package>` scope in `pandabox.wxs`:
```xml
<Package InstallScope="perUser">
```

### Custom Install Dialog

Replace in `pandabox.wxs`:
```xml
<ui:WixUI Id="WixUI_InstallDir" InstallDirectory="INSTALLFOLDER" />
```

With other options:
- `WixUI_Minimal` - Simplest UI
- `WixUI_Mondo` - Full customization
- `WixUI_FeatureTree` - Feature selection

## Automation & CI/CD

### GitHub Actions Example

```yaml
name: Build Windows Installer

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Install WiX
        run: dotnet tool install --global wix
        
      - name: Build Installer
        run: |
          cd distribute
          .\build-installer.ps1
          
      - name: Upload Installer
        uses: actions/upload-artifact@v3
        with:
          name: Pandabox-Installer
          path: distribute/output/*.msi
```

### Batch Script Wrapper

Create `build.bat` for simpler execution:
```batch
@echo off
cd distribute
powershell -ExecutionPolicy Bypass -File build-installer.ps1 %*
pause
```

## Next Steps

1. **Test on clean Windows VM** - Ensure all dependencies are bundled
2. **Create release checklist** - Document your release process
3. **Set up code signing** - For production releases
4. **Add auto-update mechanism** - Check for updates on launch
5. **Create silent install guide** - For enterprise deployment

## Resources

- [WiX Tutorial](https://www.firegiant.com/wix/tutorial/)
- [MSI Best Practices](https://docs.microsoft.com/en-us/windows/win32/msi/windows-installer-best-practices)
- [Rust Windows Deployment](https://rust-lang.github.io/rustup/installation/windows.html)

## Support

If you encounter issues:

1. Check `distribute/output/*.log` for build errors
2. Run with verbose output: `.\build-installer.ps1 -Verbose`
3. Open an issue on GitHub with:
   - Error message
   - PowerShell version: `$PSVersionTable.PSVersion`
   - WiX version: `wix --version`
   - Windows version: `winver`
