project(
  'tuxsplit',
  'rust',
  version : '0.1.0',
  default_options : [
    'warning_level=2'
  ]
)

# Build resources and copy them to the appropriate location
gnome = import('gnome')
glib_compile_resources = find_program('glib-compile-resources', required : true)
tuxsplit_resources = custom_target(
  'tuxsplit-resources',
  input : meson.project_source_root() / 'data/tuxsplit.xml',
  output : 'tuxsplit.gresource',
  command : [
    glib_compile_resources,
    '--sourcedir=@SOURCE_ROOT@/data',
    '--target=@OUTPUT@',
    '@INPUT@'
  ],
  build_by_default : true,
  install : true,
  install_dir : join_paths(get_option('datadir'), 'tuxsplit')
)

# Move the prebuilt cargo binary to the appropriate location
cargo_bin_path = meson.project_source_root() / 'target/release/tuxsplit'
install_data(
  cargo_bin_path,
  install_dir : get_option('bindir'),
  install_tag : 'runtime'
)

# Default config
install_data(
  'data/config/config.yaml',
  install_dir : join_paths(get_option('datadir'), 'tuxsplit', 'config'),
  install_tag : 'data'
)
