project('musicus', 'rust',
    version: '0.1.0',
    meson_version: '>= 0.50.0',
    license: 'AGPLv3+',
)

dependency('dbus-1', version: '>= 1.3')
dependency('glib-2.0', version: '>= 2.56')
dependency('gio-2.0', version: '>= 2.56')
dependency('gstreamer-1.0', version: '>= 1.12')
dependency('gtk4', version: '>= 4.0')
#dependency('libcurl', version: '>= 7.24.0')
dependency('libdiscid', version: '>= 0.6.2')
dependency('libadwaita-1', version: '>= 1.0')
dependency('pango', version: '>= 1.0')
dependency('sqlite3', version: '>= 3.20')

i18n = import('i18n')

subdir('data')
subdir('po')
subdir('res')
subdir('crates/musicus/src')

meson.add_install_script('build-aux/postinstall.py')
