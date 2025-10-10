# Building Pandabox Flatpak

This guide explains how to build and distribute Pandabox as a Flatpak.

## Prerequisites

Install Flatpak and flatpak-builder:

```bash
# Debian/Ubuntu
sudo apt install flatpak flatpak-builder python3-pip

# Fedora
sudo dnf install flatpak flatpak-builder python3-pip

# Arch
sudo pacman -S flatpak flatpak-builder python-pip
```

Install Python dependencies:
```bash
pip3 install --user aiohttp toml
```

Add Flathub repository:
```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

## Quick Build

Simply run the build script:

```bash
./build-flatpak.sh
```

This will:
1. Install required Flatpak runtimes
2. Generate Cargo sources for offline build
3. Build and install the Flatpak locally

## Manual Build Steps

If you prefer to build manually:

### 1. Generate Cargo Sources

```bash
# Download the generator if needed
wget https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py

# Generate sources
python3 flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
```

### 2. Install Runtime and SDK

```bash
flatpak install flathub org.freedesktop.Platform//24.08
flatpak install flathub org.freedesktop.Sdk//24.08
flatpak install flathub org.freedesktop.Sdk.Extension.rust-stable//24.08
```

### 3. Build the Flatpak

```bash
flatpak-builder --force-clean --user --install build-dir com.github.mastrHyperion98.Pandabox.yml
```

## Running the Application

After building:

```bash
flatpak run com.github.mastrHyperion98.Pandabox
```

## Creating a Distributable Bundle

To create a `.flatpak` file for distribution:

```bash
flatpak build-bundle ~/.local/share/flatpak/repo pandabox.flatpak com.github.mastrHyperion98.Pandabox
```

Users can install it with:
```bash
flatpak install pandabox.flatpak
```

## Publishing to Flathub

To publish on Flathub:

1. Fork the [Flathub repository](https://github.com/flathub/flathub)
2. Create a new repository: `com.github.mastrHyperion98.Pandabox`
3. Add the manifest files
4. Submit a pull request to Flathub

See [Flathub submission guidelines](https://docs.flathub.org/docs/for-app-authors/submission/) for details.

## Troubleshooting

### Build fails with network errors
The Flatpak build is offline by default. Make sure `cargo-sources.json` is generated correctly.

### Missing dependencies
Ensure all Flatpak runtimes are installed:
```bash
flatpak list --runtime
```

### Permission issues
The app needs home directory access for the database. This is configured in the manifest with:
```yaml
finish-args:
  - --filesystem=home
```

## File Structure

- `com.github.mastrHyperion98.Pandabox.yml` - Flatpak manifest
- `com.github.mastrHyperion98.Pandabox.desktop` - Desktop entry
- `com.github.mastrHyperion98.Pandabox.metainfo.xml` - AppStream metadata
- `cargo-sources.json` - Generated Cargo dependencies (created during build)
- `build-flatpak.sh` - Automated build script
