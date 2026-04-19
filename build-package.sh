#!/bin/bash
# Build Aethermap pacman package
set -e

echo "========================================="
echo "  Building Aethermap Pacman Package"
echo "========================================="

# Check if we're in the right directory
if [ ! -f "PKGBUILD" ]; then
    echo "ERROR: PKGBUILD not found. Run this script from the aethermap directory."
    exit 1
fi

# Check dependencies
echo "Checking build dependencies..."
if ! command -v cargo &> /dev/null; then
    echo "ERROR: cargo not found. Install rust with: pacman -S rust"
    exit 1
fi

if ! command -v makepkg &> /dev/null; then
    echo "ERROR: makepkg not found. Install base-devel with: pacman -S base-devel"
    exit 1
fi

# Clean previous builds
echo "Cleaning previous builds..."
rm -f *.pkg.tar.zst *.pkg.tar.xz 2>/dev/null || true

# Build release binaries first
echo "Building release binaries..."
cargo build --release -p aethermapd -p aethermap-gui

# Build the package
echo "Building pacman package..."
makepkg -f

# Show the result
echo ""
echo "========================================="
echo "  Build Complete!"
echo "========================================="
echo ""
ls -lh *.pkg.tar.* 2>/dev/null || echo "No package file found"
echo ""
echo "To install:"
echo "  sudo pacman -U aethermap-*.pkg.tar.zst"
echo ""
echo "To remove:"
echo "  sudo pacman -R aethermap"
echo ""
