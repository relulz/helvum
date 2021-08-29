project(
  'helvum',
  'rust',
  version: '0.3.0',
  license: 'GPL-3.0',
  meson_version: '>=0.50.0'
)

dependency('glib-2.0', version: '>= 2.48')
dependency('gtk4',     version: '>= 4.0.0')
dependency('libpipewire-0.3')

rust_version = meson.get_compiler('rust').version()
min_rust_version = '1.54.0'
if rust_version < min_rust_version
  error('Rust version too old: Required version is ' + min_rust_version + ' but actual version is ' + rust_version)
endif

cargo = find_program('cargo', required: true)
cargo_script = find_program('build-aux/cargo.sh')

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')

cargo_sources = files(
  'Cargo.toml',
  'Cargo.lock',
)

subdir('src')
