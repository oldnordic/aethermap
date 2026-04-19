# Maintainer: Aethermap Team
pkgname=aethermap
pkgver=1.0.0
pkgrel=1
pkgdesc="Wayland-compatible input device remapper for Linux with per-device profiles and hotplug support"
arch=('x86_64')
url="https://github.com/yourusername/aethermap"
license=('MIT' 'Apache-2.0')
depends=('gcc-libs' 'systemd')
makedepends=('cargo')
provides=('aethermap' 'aethermapd' 'aethermap-gui')
conflicts=('aethermap' 'aethermapd' 'aethermap-gui')
source=("aethermap-$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    # Create a tarball of the current source
    cd "$srcdir/.."
    tar czf "$srcdir/aethermap-$pkgver.tar.gz" \
        --exclude='target' \
        --exclude='*.pkg.tar.*' \
        --exclude='.git' \
        --exclude='fastembed_cache' \
        --exclude='syncore.*' \
        --exclude='vector.index.*' \
        --exclude='.fastembed_cache' \
        --exclude='backups' \
        aethermap/ Cargo.* PKGBUILD README.md .planning/
}

build() {
    cd "$srcdir/aethermap-$pkgver/aethermap"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=../../target
    cargo build --release --all-features
}

check() {
    cd "$srcdir/aethermap-$pkgver/aethermap"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --release --all-features --no-fail-fast -- --skip test_macro_playback
}

package() {
    cd "$srcdir/aethermap-$pkgver"

    # Install binaries
    install -Dm755 target/release/aethermapd "$pkgdir/usr/bin/aethermapd"
    install -Dm755 target/release/aethermap-gui "$pkgdir/usr/bin/aethermap-gui"

    # Install systemd service
    install -Dm644 aethermap/aethermapd/systemd/aethermapd.service "$pkgdir/usr/lib/systemd/system/aethermapd.service"

    # Install desktop file for GUI
    install -Dm644 aethermap/aethermap-gui.desktop "$pkgdir/usr/share/applications/aethermap-gui.desktop"

    # Install documentation
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"

    # Create config directory
    install -dm755 "$pkgdir/etc/aethermap"
}
