# Maintainer: Razermapper Team
pkgname=razermapper
pkgver=1.0.0
pkgrel=1
pkgdesc="Wayland-compatible input device remapper for Linux with per-device profiles and hotplug support"
arch=('x86_64')
url="https://github.com/yourusername/razermapper"
license=('MIT' 'Apache-2.0')
depends=('gcc-libs' 'systemd')
makedepends=('cargo')
provides=('razermapper' 'razermapperd' 'razermapper-gui')
source=("razermapper-$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    # Create a tarball of the current source
    cd "$srcdir/.."
    tar czf "$srcdir/razermapper-$pkgver.tar.gz" \
        --exclude='target' \
        --exclude='*.pkg.tar.*' \
        --exclude='.git' \
        --exclude='fastembed_cache' \
        --exclude='syncore.*' \
        --exclude='vector.index.*' \
        --exclude='.fastembed_cache' \
        --exclude='backups' \
        razermapper/ Cargo.* PKGBUILD README.md .planning/
}

build() {
    cd "$srcdir/razermapper-$pkgver/razermapper"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=../../target
    cargo build --release --all-features
}

check() {
    cd "$srcdir/razermapper-$pkgver/razermapper"
    export RUSTUP_TOOLCHAIN=stable
    cargo test --release --all-features --no-fail-fast -- --skip test_macro_playback
}

package() {
    cd "$srcdir/razermapper-$pkgver"

    # Install binaries
    install -Dm755 target/release/razermapperd "$pkgdir/usr/bin/razermapperd"
    install -Dm755 target/release/razermapper-gui "$pkgdir/usr/bin/razermapper-gui"

    # Install systemd service
    install -Dm644 razermapper/razermapperd/systemd/razermapperd.service "$pkgdir/usr/lib/systemd/system/razermapperd.service"

    # Install desktop file for GUI
    install -Dm644 razermapper/razermapper.desktop "$pkgdir/usr/share/applications/razermapper-gui.desktop"

    # Install documentation
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"

    # Create config directory
    install -dm755 "$pkgdir/etc/razermapperd"
}
