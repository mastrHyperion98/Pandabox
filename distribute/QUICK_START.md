# Quick Start Guide - Windows Installer

## ⚡ TL;DR

```powershell
# 1. Install WiX
dotnet tool install --global wix

# 2. Build installer
cd distribute
.\build-installer.ps1

# 3. Find installer
# Output: distribute\output\Pandabox-0.1.1.msi
```

## 📋 Prerequisites Checklist

- [ ] Rust installed (`cargo --version`)
- [ ] WiX Toolset v4 installed (`wix --version`)
- [ ] .NET SDK installed (for WiX)

## 🚀 Common Commands

### Build Full Release
```powershell
cd distribute
.\build-installer.ps1
```

### Build Without Recompiling
```powershell
.\build-installer.ps1 -SkipBuild
```

### Clean Build
```powershell
.\build-installer.ps1 -Clean
```

### Test Installer
```powershell
# Show info
.\test-installer.ps1 -Info

# Install locally
.\test-installer.ps1 -Install

# Uninstall
.\test-installer.ps1 -Uninstall
```

### Collect Dependencies Only
```powershell
.\collect-dependencies.ps1
```

## 📁 Output Location

```
distribute/
├── output/
│   └── Pandabox-0.1.1.msi  ← Your installer is here!
└── staging/                 ← Temporary build files
```

## 🎨 Optional: Add Branding

Create these files in `distribute/assets/`:

```
assets/
├── icon.ico      (256x256)
├── banner.bmp    (493x58)
└── dialog.bmp    (493x312)
```

## ❓ Troubleshooting

### "wix: command not found"
```powershell
dotnet tool install --global wix
$env:PATH += ";$env:USERPROFILE\.dotnet\tools"
```

### "Execution Policy" Error
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

### Missing DLLs After Install
Install [VC++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)

## 📚 More Info

- **Full Documentation**: [README.md](README.md)
- **Setup Guide**: [SETUP.md](SETUP.md)
- **Main Distribution Guide**: [../DISTRIBUTION.md](../DISTRIBUTION.md)

## 🎯 What You Get

✅ Professional MSI installer  
✅ Desktop shortcut  
✅ Start Menu entry  
✅ Proper Windows uninstaller  
✅ All dependencies bundled  
✅ Registry integration  

## 🔄 Release Workflow

1. Update version in `Cargo.toml`
2. Update version in `pandabox.wxs`
3. Run `.\build-installer.ps1`
4. Test with `.\test-installer.ps1 -Install`
5. Upload `output\Pandabox-X.X.X.msi` to GitHub Releases

---

**Need help?** Check [README.md](README.md) for detailed instructions.
