rust_sources = files(
  'application.rs',
  'main.rs',
  'pipewire_connection.rs',
  'pipewire_connection/state.rs',
  'view/graph_view.rs',
  'view/mod.rs',
  'view/node.rs',
  'view/port.rs',
)

custom_target(
  'cargo-build',
  build_by_default: true,
  input: [
    cargo_sources,
    rust_sources
  ],
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: bindir,
  command: [
    cargo_script,
    meson.build_root(),
    meson.source_root(),
    '@OUTPUT@',
    get_option('profile'),
    meson.project_name(),
  ],
)
