# Configuration file

conf = configuration_data()
conf.set_quoted('NAME', name)
conf.set_quoted('PKGNAME', meson.project_name())
conf.set_quoted('APP_ID', app_id)
conf.set_quoted('PATH_ID', path_id)
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('PROFILE', profile)
conf.set_quoted('LOCALEDIR', localedir)
conf.set_quoted('DATADIR', datadir)
conf.set_quoted('METADATA_URL', metadata_url)
conf.set_quoted('LIBRARY_URL', library_url)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf,
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  meson.project_build_root() / 'src' / 'config.rs',
  meson.project_source_root() / 'src' / 'config.rs',
  check: true
)

# Cargo

cargo_bin  = find_program('cargo')
sh_bin     = find_program('sh')
exe_suffix = host_machine.system() == 'windows' ? '.exe' : ''
exe_name   = meson.project_name() + exe_suffix
cargo_opt  = [ '--manifest-path', meson.project_source_root() / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root()  / 'src' ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root()  / 'cargo-home' ]

if get_option('profile') == 'release'
  cargo_opt += [ '--release' ]
  rust_target = 'release'
  message('Building in release mode')
else
  rust_target = 'debug'
  message('Building in debug mode')
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: exe_name,
  console: true,
  install: true,
  install_dir: bindir,
  env: cargo_env,
  command: [
    sh_bin, meson.project_source_root() / 'src' / 'cargo-build.sh',
    cargo_bin.full_path(),
    meson.project_build_root() / 'src' / rust_target / exe_name,
    '@OUTPUT@',
    cargo_opt,
  ],
)
