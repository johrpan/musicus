prefix = get_option('prefix')
localedir = join_paths(prefix, get_option('localedir'))

global_conf = configuration_data()
global_conf.set_quoted('LOCALEDIR', localedir)
global_conf.set_quoted('VERSION', meson.project_version())
config_rs = configure_file(
  input: 'config.rs.in',
  output: 'config.rs',
  configuration: global_conf
)

run_command(
  'cp',
  config_rs,
  meson.current_source_dir(),
  check: true
)

resource_conf = configuration_data()
resource_conf.set_quoted('RESOURCEFILE', resources.full_path())
resource_rs = configure_file(
  input: 'resources.rs.in',
  output: 'resources.rs',
  configuration: resource_conf
)

run_command(
  'cp',
  resource_rs,
  meson.current_source_dir(),
  check: true
)

sources = files(
  'config.rs',
  'resources.rs',
)

cargo_script = find_program(join_paths(meson.source_root(), 'build-aux/cargo.sh'))
cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
  build_always_stale: true,
  depends: resources,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    cargo_script,
    meson.build_root(),
    meson.source_root(),
    '@OUTPUT@',
    get_option('buildtype'),
    meson.project_name(),
  ]
)
