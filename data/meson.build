datadir = get_option('datadir')

scalable_dir = join_paths('icons', 'hicolor', 'scalable', 'apps')
install_data(
  join_paths(scalable_dir, 'de.johrpan.musicus.svg'),
  install_dir: join_paths(datadir, scalable_dir),
)

symbolic_dir = join_paths('icons', 'hicolor', 'symbolic', 'apps')
install_data(
  join_paths(symbolic_dir, 'de.johrpan.musicus-symbolic.svg'),
  install_dir: join_paths(datadir, symbolic_dir),
)

desktop_file = i18n.merge_file(
  input: 'de.johrpan.musicus.desktop.in',
  output: 'de.johrpan.musicus.desktop',
  type: 'desktop',
  po_dir: '../po',
  install: true,
  install_dir: join_paths(datadir, 'applications')
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils,
    args: [desktop_file]
  )
endif

install_data('de.johrpan.musicus.gschema.xml',
  install_dir: join_paths(get_option('datadir'), 'glib-2.0/schemas')
)

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file', compile_schemas,
    args: ['--strict', '--dry-run', meson.current_source_dir()]
  )
endif
