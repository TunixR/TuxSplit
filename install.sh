cargo build --release;
meson setup build;
meson compile -C build;
sudo meson install -C build;
