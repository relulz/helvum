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


appdata_conf = configuration_data()
appdata_conf.set('app-id', base_id)
appdata_file = configure_file(
  input: '@0@.metainfo.xml.in'.format(base_id),
  output: '@BASENAME@',
  configuration: appdata_conf
)

# Validate Appdata
if appstream_util.found()
  test(
    'validate-appdata',
    appstream_util,
    args: [
      'validate', '--nonet', appdata_file
    ],
  )
endif

install_data(
  appdata_file,
  install_dir: datadir / 'metainfo'
)
