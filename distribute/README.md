# Pandabox Windows Distribution

This directory contains scripts and configuration files for building Windows installers for Pandabox.

## 🎯 Quick Start

### Prerequisites

1. **Rust Toolchain**
   ```powershell
   # Install from https://rustup.rs/
   # Or check if installed:
   cargo --version
   ```

2. **WiX Toolset v4**
   ```powershell
   # Install via .NET tool
   dotnet tool install --global wix
   
   # Verify installation
   wix --version
   ```

3. **Visual Studio Build Tools** (for MSVC runtime)
   - Download from: https://visualstudio.microsoft.com/downloads/
   - Select "Desktop development with C++" workload

### Build Installer

```powershell
# Navigate to distribute directory
cd distribute

# Build everything (compile + create installer)
.\build-installer.ps1

# Or skip compilation if binary already exists
.\build-installer.ps1 -SkipBuild

# Clean previous builds
.\build-installer.ps1 -Clean
```

The installer will be created in `distribute/output/Pandabox-{version}.msi`

## 📦 What Gets Installed

The installer includes:

- ✅ **Pandabox.exe** - Main application binary
- ✅ **All required DLLs** - Slint runtime, MSVC runtime, etc.
- ✅ **Desktop shortcut** - Quick access icon
- ✅ **Start Menu entry** - Under "Pandabox" folder
- ✅ **Uninstaller** - Proper Windows uninstall support
- ✅ **Registry entries** - For proper integration

### Installation Location

Default: `C:\Program Files\Pandabox\`

Users can choose a custom location during installation.

## 🛠️ Customization

### Branding Assets

Place these files in `distribute/assets/`:

1. **icon.ico** (256x256 pixels)
   - Application icon shown in shortcuts and Add/Remove Programs
   
2. **banner.bmp** (493x58 pixels)
   - Top banner in installer dialogs
   
3. **dialog.bmp** (493x312 pixels)
   - Background image in installer dialogs

If these files are missing, the installer will use Windows defaults.

### Version Management

Update version in three places:

1. `Cargo.toml` - Line 3
   ```toml
   version = "0.1.1"
   ```

2. `pandabox.wxs` - Line 6
   ```xml
   Version="0.1.1.0"
   ```

3. `build-installer.ps1` - Line 5 (default parameter)
   ```powershell
   [string]$Version = "0.1.1"
   ```

### Upgrade Code

The `UpgradeCode` in `pandabox.wxs` should **NEVER** change. This GUID allows Windows to recognize updates to the same product.

Current UpgradeCode: `A1B2C3D4-E5F6-4A5B-8C9D-0E1F2A3B4C5D`

## 📋 File Structure

```
distribute/
├── pandabox.wxs              # Main WiX installer configuration
├── dependencies.wxs          # Auto-generated dependency manifest
├── build-installer.ps1       # Main build script
├── collect-dependencies.ps1  # Dependency collection helper
├── README.md                 # This file
├── assets/                   # Branding assets (icons, images)
│   ├── icon.ico
│   ├── banner.bmp
│   ├── dialog.bmp
│   └── license.rtf          # Auto-generated license
├── staging/                  # Temporary build files (auto-created)
└── output/                   # Final MSI installers (auto-created)
```

## 🔧 Advanced Usage

### Manual Dependency Collection

```powershell
.\collect-dependencies.ps1 -BinaryPath "..\target\release\Pandabox.exe" -OutputDir "staging"
```

### Install MSI Silently

```powershell
# Silent install
msiexec /i "output\Pandabox-0.1.1.msi" /quiet

# Silent install with log
msiexec /i "output\Pandabox-0.1.1.msi" /quiet /l*v install.log
```

### Uninstall Silently

```powershell
# Using product code (found in registry)
msiexec /x {PRODUCT-CODE-GUID} /quiet

# Or using the MSI file
msiexec /x "output\Pandabox-0.1.1.msi" /quiet
```

## 🐛 Troubleshooting

### "WiX not found"

Install WiX Toolset:
```powershell
dotnet tool install --global wix
```

Add to PATH if needed:
```powershell
$env:PATH += ";$env:USERPROFILE\.dotnet\tools"
```

### "Missing DLLs" Error

The application may need additional runtime DLLs. Options:

1. **Install Visual C++ Redistributable**
   - Download from Microsoft
   - Users need this installed

2. **Bundle DLLs** (current approach)
   - Script automatically collects from System32
   - Some DLLs may require admin rights to copy

3. **Static linking** (future option)
   - Add to `Cargo.toml`:
     ```toml
     [profile.release]
     rustflags = ["-C", "target-feature=+crt-static"]
     ```

### "Access Denied" During Build

Run PowerShell as Administrator or adjust execution policy:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Installer Shows Wrong Version

Ensure version numbers match in:
- `Cargo.toml`
- `pandabox.wxs`
- Build script parameter

## 📝 Distribution Checklist

Before releasing:

- [ ] Update version in all files
- [ ] Test build on clean Windows machine
- [ ] Verify all shortcuts work
- [ ] Test uninstaller
- [ ] Check Add/Remove Programs entry
- [ ] Verify application runs after install
- [ ] Test upgrade from previous version
- [ ] Create GitHub release with MSI
- [ ] Update CHANGELOG.md
- [ ] Tag release in git

## 🔐 Security Notes

- MSI files are not signed by default
- Consider code signing for production releases
- Users may see SmartScreen warnings without signature
- Database stored in: `%LOCALAPPDATA%\pandabox\`

## 📚 Additional Resources

- [WiX Toolset Documentation](https://wixtoolset.org/docs/)
- [Windows Installer Best Practices](https://docs.microsoft.com/en-us/windows/win32/msi/windows-installer-best-practices)
- [Rust Windows Deployment](https://doc.rust-lang.org/cargo/reference/manifest.html#the-profile-sections)

## 🤝 Contributing

To improve the installer:

1. Test on different Windows versions (10, 11)
2. Add support for per-user vs. system-wide installation
3. Implement automatic update checking
4. Add file associations (.pbox files?)
5. Create silent install options for enterprise deployment

## 📄 License

Same as Pandabox - MIT License
