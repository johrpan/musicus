project(
  'musicus',
  'rust', 
  version: '0.1.0',
  meson_version: '>= 0.62.0',
)

i18n = import('i18n')
gnome = import('gnome')

dependency('libadwaita-1', version: '>= 1.7')
dependency('sqlite3', version: '>= 3.38')
dependency('glib-2.0', version: '>= 2.84')
dependency('gstreamer-1.0', version: '>= 1.16')
dependency('gstreamer-play-1.0', version: '>= 1.16')
dependency('gtk4', version: '>= 4.18')
dependency('dbus-1')
dependency('openssl', version: '>= 1.0')

name = 'Musicus'
base_id = 'de.johrpan.Musicus'
library_url = 'https://musicus.johrpan.de/musicus_library_latest.zip'
app_id = base_id
path_id = '/de/johrpan/Musicus'
profile = get_option('profile')
datadir = get_option('prefix') / get_option('datadir')
localedir = get_option('prefix') / get_option('localedir')
bindir = get_option('prefix') / get_option('bindir')

if profile == 'development'
  app_id = '@0@.Devel'.format(app_id)
endif

subdir('data')
subdir('src')
subdir('po')

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
