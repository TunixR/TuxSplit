cargo build --release;
meson setup build --datadir=/usr/share/tuxsplit;
meson compile -C build;
sudo meson install -C build;
