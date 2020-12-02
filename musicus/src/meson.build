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
  'backend/client/mod.rs',
  'backend/client/ensembles.rs',
  'backend/client/instruments.rs',
  'backend/client/persons.rs',
  'backend/client/recordings.rs',
  'backend/client/works.rs',
  'backend/library.rs',
  'backend/mod.rs',
  'backend/secure.rs',
  'database/ensembles.rs',
  'database/instruments.rs',
  'database/mod.rs',
  'database/persons.rs',
  'database/recordings.rs',
  'database/schema.rs',
  'database/thread.rs',
  'database/tracks.rs',
  'database/works.rs',
  'dialogs/about.rs',
  'dialogs/login_dialog.rs',
  'dialogs/mod.rs',
  'dialogs/preferences.rs',
  'dialogs/server_dialog.rs',
  'editors/ensemble.rs',
  'editors/instrument.rs',
  'editors/mod.rs',
  'editors/performance.rs',
  'editors/person.rs',
  'editors/recording.rs',
  'editors/track.rs',
  'editors/tracks.rs',
  'editors/work.rs',
  'editors/work_part.rs',
  'editors/work_section.rs',
  'screens/ensemble_screen.rs',
  'screens/mod.rs',
  'screens/person_screen.rs',
  'screens/player_screen.rs',
  'screens/recording_screen.rs',
  'screens/work_screen.rs',
  'selectors/ensemble.rs',
  'selectors/instrument.rs',
  'selectors/mod.rs',
  'selectors/person.rs',
  'selectors/recording.rs',
  'selectors/selector.rs',
  'selectors/work.rs',
  'widgets/list.rs',
  'widgets/mod.rs',
  'widgets/navigator.rs',
  'widgets/navigator_window.rs',
  'widgets/player_bar.rs',
  'widgets/poe_list.rs',
  'widgets/selector_row.rs',
  'config.rs',
  'config.rs.in',
  'main.rs',
  'player.rs',
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
