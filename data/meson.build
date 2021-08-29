desktop_file = '@0@.desktop'.format(base_id)

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
