# Pandabox Distribution Guide

## 🪟 Windows Installer (MSI)

### Quick Start

```powershell
cd distribute
.\build-installer.ps1
```

This builds a professional Windows MSI installer with:
- Desktop shortcut
- Start Menu entry
- Proper uninstall support
- All dependencies bundled

### Installation

Users can install by:
1. Double-clicking the MSI file
2. Or via command line:
   ```powershell
   msiexec /i Pandabox-0.1.1.msi
   ```

### Distribution Options

#### 1. **Direct Download** (GitHub Releases)
- Upload MSI to GitHub releases
- Users download and install directly
- Simple and straightforward

#### 2. **Microsoft Store** (future)
- Wider reach and automatic updates
- Requires Microsoft Partner account
- See [Microsoft Store submission guide](https://docs.microsoft.com/en-us/windows/apps/publish/)

#### 3. **Chocolatey** (package manager)
- Popular Windows package manager
- Users install with: `choco install pandabox`
- See [Chocolatey package creation](https://docs.chocolatey.org/en-us/create/create-packages)

#### 4. **Winget** (Windows Package Manager)
- Microsoft's official package manager
- Users install with: `winget install Pandabox`
- Submit to [winget-pkgs repository](https://github.com/microsoft/winget-pkgs)

See [distribute/README.md](distribute/README.md) for detailed instructions.

## 📦 Flatpak (Linux)

### Quick Start

```bash
./build-flatpak.sh
```

This builds and installs Pandabox as a Flatpak on your system.

### Distribution Options

#### 1. **Local Bundle** (for sharing with users)
```bash
flatpak build-bundle ~/.local/share/flatpak/repo pandabox.flatpak com.github.mastrHyperion98.Pandabox
```
Share the `pandabox.flatpak` file. Users install with:
```bash
flatpak install pandabox.flatpak
```

#### 2. **Flathub** (recommended for wide distribution)
- Fork https://github.com/flathub/flathub
- Create repository: `com.github.mastrHyperion98.Pandabox`
- Submit PR with manifest files
- Once approved, users can install via:
  ```bash
  flatpak install flathub com.github.mastrHyperion98.Pandabox
  ```

#### 3. **Custom Repository**
Host your own Flatpak repository:
```bash
flatpak build-export repo build-dir
flatpak build-update-repo repo
```

See [FLATPAK_BUILD.md](FLATPAK_BUILD.md) for detailed instructions.

## 🍎 macOS (Future)

### DMG Installer
- Drag-and-drop installation
- Code signed for Gatekeeper
- Notarized by Apple

### Homebrew
```bash
brew install pandabox
```

## 🚀 Additional Distribution Methods

### Linux

#### AppImage
- Single executable file
- No installation required
- Works on any Linux distribution

#### Snap
- Canonical's universal package format
- Automatic updates
- Snap Store distribution

#### Traditional Packages
- **.deb** (Debian/Ubuntu)
- **.rpm** (Fedora/RHEL)
- **AUR** (Arch Linux)

### Cross-Platform

#### Cargo/crates.io
```bash
cargo install pandabox
```

## 📝 Release Checklist

Before creating a release:

### Version Updates
- [ ] Update version in `Cargo.toml`
- [ ] Update version in `com.github.mastrHyperion98.Pandabox.metainfo.xml` (Linux)
- [ ] Update version in `distribute/pandabox.wxs` (Windows)
- [ ] Update `CHANGELOG.md`

### Testing
- [ ] Run tests: `cargo test`
- [ ] Build release: `cargo build --release`
- [ ] Test Windows installer: `cd distribute && .\build-installer.ps1`
- [ ] Test Flatpak build: `./build-flatpak.sh`

### Release
- [ ] Create git tag: `git tag v0.1.1`
- [ ] Push tag: `git push origin v0.1.1`
- [ ] Create GitHub release with:
  - Windows MSI installer
  - Linux Flatpak bundle (optional)
  - Release notes from CHANGELOG

## 🔐 Security Notes

### Data Storage
- **Windows**: `%LOCALAPPDATA%\pandabox\`
- **Linux**: `~/.local/share/pandabox/`
- **Flatpak**: Has `--filesystem=home` permission for database access

### Encryption
- Master password never leaves the device
- All data encrypted with ChaCha20-Poly1305
- Passwords hashed with Argon2

### Code Signing
- Windows MSI is unsigned by default (shows SmartScreen warning)
- For production, consider purchasing a code signing certificate
- Linux Flatpak uses GPG signatures

## 📊 Metrics

Track downloads via:
- GitHub release downloads (Windows MSI, Linux bundles)
- Flathub statistics (if published)
- Microsoft Store analytics (if published)
- Chocolatey/Winget statistics
- Cargo crate downloads (if published)

## 🛠️ Platform-Specific Guides

- **Windows**: See [distribute/README.md](distribute/README.md) and [distribute/SETUP.md](distribute/SETUP.md)
- **Linux**: See [FLATPAK_BUILD.md](FLATPAK_BUILD.md)
