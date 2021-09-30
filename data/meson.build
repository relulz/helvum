subdir('icons')

desktop_conf = configuration_data()
desktop_conf.set('icon', base_id)
desktop_file = configure_file(
  input: '@0@.desktop.in'.format(base_id),
  output: '@BASENAME@',
  configuration: desktop_conf
)

if desktop_file_validate.found()
  test(
    'validate-desktop',
    desktop_file_validate,
    args: [
      desktop_file
    ],
  )
endif

install_data(
  desktop_file,
  install_dir: datadir / 'applications'
)
