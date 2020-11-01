project('musicus', 'rust',
    version: '0.1.0',
    meson_version: '>= 0.50.0',
    license: 'AGPLv3+',
)

dependency('glib-2.0', version: '>= 2.56')
dependency('gio-2.0', version: '>= 2.56')
dependency('gtk+-3.0', version: '>= 3.24.7')
dependency('libhandy-1', version: '>= 1.0.0')
dependency('pango', version: '>= 1.0')
dependency('sqlite3', version: '>= 3.20')

i18n = import('i18n')

subdir('data')
subdir('po')
subdir('res')
subdir('src')

meson.add_install_script('build-aux/postinstall.py')