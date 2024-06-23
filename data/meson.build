# Desktop file

conf = configuration_data()
conf.set('NAME', name)
conf.set('APP_ID', app_id)
conf.set('PKGNAME', meson.project_name())

desktop_file = i18n.merge_file(
  input: configure_file(
    input: '@0@.desktop.in.in'.format(base_id),
    output: '@0@.desktop.in'.format(app_id),
    configuration: conf,
  ),
  output: '@0@.desktop'.format(app_id),
  type: 'desktop',
  po_dir: '../po',
  install: true,
  install_dir: datadir / 'applications',
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

# Settings

conf = configuration_data()
conf.set('APP_ID', app_id)
conf.set('PATH_ID', path_id)
conf.set('PKGNAME', meson.project_name())

configure_file(
  input: '@0@.gschema.xml.in'.format(base_id),
  output: '@0@.gschema.xml'.format(app_id),
  configuration: conf,
  install: true,
  install_dir: datadir / 'glib-2.0' / 'schemas',
)

compile_schemas = find_program('glib-compile-schemas', required: false)
if compile_schemas.found()
  test('Validate schema file', compile_schemas, args: ['--strict', '--dry-run', meson.current_source_dir()])
endif

# DBus service

conf = configuration_data()
conf.set('APP_ID', app_id)
conf.set('BINDIR', bindir)
conf.set('PKGNAME', meson.project_name())

configure_file(
  input: '@0@.service.in'.format(base_id),
  output: '@0@.service'.format(app_id),
  configuration: conf,
  install_dir: datadir / 'dbus-1' / 'services',
)

subdir('icons')
subdir('res')
