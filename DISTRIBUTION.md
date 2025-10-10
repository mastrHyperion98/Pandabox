# Pandabox Distribution Guide

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

## 🚀 Future Distribution Methods

### AppImage
- Single executable file
- No installation required
- Works on any Linux distribution

### Snap
- Canonical's universal package format
- Automatic updates
- Snap Store distribution

### Traditional Packages
- **.deb** (Debian/Ubuntu)
- **.rpm** (Fedora/RHEL)
- **AUR** (Arch Linux)

### Cargo/crates.io
```bash
cargo install pandabox
```

## 📝 Release Checklist

Before creating a release:

- [ ] Update version in `Cargo.toml`
- [ ] Update version in `com.github.mastrHyperion98.Pandabox.metainfo.xml`
- [ ] Update `CHANGELOG.md`
- [ ] Run tests: `cargo test`
- [ ] Build release: `cargo build --release`
- [ ] Test Flatpak build: `./build-flatpak.sh`
- [ ] Create git tag: `git tag v0.1.0`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] Create GitHub release with binaries

## 🔐 Security Notes

- Database stored in: `~/.local/share/pandabox/`
- Flatpak has `--filesystem=home` permission for database access
- Master password never leaves the device
- All data encrypted with ChaCha20-Poly1305

## 📊 Metrics

Track downloads via:
- GitHub release downloads
- Flathub statistics (if published)
- Cargo crate downloads (if published)
