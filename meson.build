project(
  'helvum',
  'rust',
  version: '0.3.2',
  license: 'GPL-3.0',
  meson_version: '>=0.50.0'
)

base_id = 'org.freedesktop.ryuukyu.Helvum'

dependency('glib-2.0', version: '>= 2.48')
dependency('gtk4',     version: '>= 4.4.0')
dependency('libpipewire-0.3')

desktop_file_validate = find_program('desktop-file-validate', required: false)
appstream_util = find_program('appstream-util', required: false)
cargo = find_program('cargo', required: true)
cargo_script = find_program('build-aux/cargo.sh')

prefix = get_option('prefix')
bindir = prefix / get_option('bindir')
datadir = prefix / get_option('datadir')
iconsdir = datadir / 'icons'

meson.add_dist_script(
  'build-aux/dist-vendor.sh',
  meson.build_root() / 'meson-dist' / meson.project_name() + '-' + meson.project_version(),
  meson.source_root()
)

cargo_sources = files(
  'Cargo.toml',
  'Cargo.lock',
)

subdir('src')
subdir('data')

meson.add_install_script('build-aux/meson_post_install.py')
