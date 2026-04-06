#!/bin/bash
set -e

echo "🔧 Building Pandabox Flatpak..."

# Install flatpak-builder if not present
if ! command -v flatpak-builder &> /dev/null; then
    echo "❌ flatpak-builder not found. Please install it:"
    echo "   sudo apt install flatpak-builder  # Debian/Ubuntu"
    echo "   sudo dnf install flatpak-builder   # Fedora"
    exit 1
fi

# Install required runtime and SDK
echo "📦 Installing Flatpak runtime and SDK..."
flatpak install -y flathub org.freedesktop.Platform//24.08 org.freedesktop.Sdk//24.08 org.freedesktop.Sdk.Extension.rust-stable//24.08 || true

# Generate cargo sources for offline build
echo "🦀 Generating Cargo sources..."

GENERATOR=/tmp/flatpak-cargo-generator.py

if ! command -v flatpak-cargo-generator.py &> /dev/null; then
    echo "📥 Downloading flatpak-cargo-generator..."
    wget https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py -O "$GENERATOR"
    chmod +x "$GENERATOR"
fi

# Ensure Python deps (aiohttp, toml) are available for the generator.
# Use a throwaway venv if they are not already importable (avoids sudo / PEP 668).
PYTHON=python3
if ! python3 -c "import aiohttp, tomlkit" 2>/dev/null; then
    echo "📦 Setting up Python deps in temporary venv..."
    python3 -m venv /tmp/pandabox-build-venv
    /tmp/pandabox-build-venv/bin/pip install --quiet aiohttp tomlkit
    PYTHON=/tmp/pandabox-build-venv/bin/python
fi

if command -v flatpak-cargo-generator.py &> /dev/null; then
    flatpak-cargo-generator.py Cargo.lock -o cargo-sources.json
else
    $PYTHON "$GENERATOR" Cargo.lock -o cargo-sources.json
fi

# Build the Flatpak
echo "🏗️  Building Flatpak..."
flatpak-builder --force-clean --user --install build-dir com.github.mastrHyperion98.Pandabox.yml

echo "✅ Build complete!"
echo ""
echo "To run the app:"
echo "  flatpak run com.github.mastrHyperion98.Pandabox"
echo ""
echo "To create a bundle for distribution:"
echo "  flatpak build-bundle ~/.local/share/flatpak/repo pandabox.flatpak com.github.mastrHyperion98.Pandabox"
