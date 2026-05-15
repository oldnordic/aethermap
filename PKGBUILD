# Maintainer: Luiz Spies <luiz@oldnordic.com>
pkgname=aethermap
pkgver=1.4.3
pkgrel=1
pkgdesc="Input remapper and macro engine for Linux devices (daemon + GUI)"
arch=('x86_64')
options=('!lto')
url="https://github.com/oldnordic/aethermap"
license=('GPL-2.0-only')
depends=('gcc-libs' 'libevdev' 'systemd-libs')
makedepends=('cargo' 'rust' 'pkg-config' 'libevdev' 'systemd')
install="$pkgname.install"
source=(
    "$pkgname-$pkgver.tar.gz::https://github.com/oldnordic/aethermap/archive/refs/tags/v$pkgver.tar.gz"
    '99-aethermap.rules'
)
sha256sums=('SKIP' 'SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    cargo fetch --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-$pkgver"
    cargo build --release --workspace
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --workspace --lib --quiet || true
}

package() {
    cd "$pkgname-$pkgver"

    # Binaries
    install -Dm755 "target/release/aethermapd"   "$pkgdir/usr/bin/aethermapd"
    install -Dm755 "target/release/aethermap-gui" "$pkgdir/usr/bin/aethermap-gui"

    # Systemd service
    install -Dm644 "aethermapd.service" "$pkgdir/usr/lib/systemd/system/aethermapd.service"

    # Udev rules
    install -Dm644 "$srcdir/99-aethermap.rules" "$pkgdir/usr/lib/udev/rules.d/99-aethermap.rules"

    # Desktop entry
    install -Dm644 "aethermap-gui.desktop" "$pkgdir/usr/share/applications/aethermap-gui.desktop"

    # License
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"

    # Config directory
    install -dm755 "$pkgdir/etc/aethermap"
}
