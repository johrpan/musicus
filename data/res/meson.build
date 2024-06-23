conf = configuration_data()
conf.set('PATH_ID', path_id)

configure_file(
  input: '@0@.gresource.xml.in'.format(base_id),
  output: '@0@.gresource.xml'.format(app_id),
  configuration: conf,
)

gnome.compile_resources(
  app_id,
  meson.project_build_root() / 'data' / 'res'/ '@0@.gresource.xml'.format(app_id),
  gresource_bundle: true,
  install: true,
  install_dir: datadir / meson.project_name(),
)