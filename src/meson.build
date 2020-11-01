prefix = get_option('prefix')
localedir = join_paths(prefix, get_option('localedir'))

global_conf = configuration_data()
global_conf.set_quoted('LOCALEDIR', localedir)
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
  'database/database.rs',
  'database/mod.rs',
  'database/models.rs',
  'database/schema.rs',
  'database/tables.rs',
  'dialogs/ensemble_editor.rs',
  'dialogs/ensemble_selector.rs',
  'dialogs/instrument_editor.rs',
  'dialogs/instrument_selector.rs',
  'dialogs/mod.rs',
  'dialogs/part_editor.rs',
  'dialogs/performance_editor.rs',
  'dialogs/person_editor.rs',
  'dialogs/person_selector.rs',
  'dialogs/preferences.rs',
  'dialogs/recording_editor.rs',
  'dialogs/recording_selector.rs',
  'dialogs/section_editor.rs',
  'dialogs/track_editor.rs',
  'dialogs/tracks_editor.rs',
  'dialogs/work_editor.rs',
  'dialogs/work_selector.rs',
  'screens/ensemble_screen.rs',
  'screens/mod.rs',
  'screens/person_screen.rs',
  'screens/recording_screen.rs',
  'screens/work_screen.rs',
  'widgets/list.rs',
  'widgets/mod.rs',
  'widgets/navigator.rs',
  'widgets/person_list.rs',
  'widgets/poe_list.rs',
  'widgets/selector_row.rs',
  'backend.rs',
  'main.rs',
  'resources.rs',
  'resources.rs.in',
  'window.rs',
)

cargo_script = find_program(join_paths(meson.source_root(), 'build-aux/cargo.sh'))
cargo_release = custom_target(
  'cargo-build',
  build_by_default: true,
  input: sources,
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
