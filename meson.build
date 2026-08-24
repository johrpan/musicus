project(
  'musicus',
  'rust', 
  version: '0.2.0',
  meson_version: '>= 0.62.0',
)

i18n = import('i18n')
gnome = import('gnome')

dependency('libadwaita-1', version: '>= 1.8')
dependency('sqlite3', version: '>= 3.38')
dependency('glib-2.0', version: '>= 2.88')
dependency('gstreamer-1.0', version: '>= 1.16')
dependency('gstreamer-play-1.0', version: '>= 1.20')
dependency('gtk4', version: '>= 4.20')

find_program('blueprint-compiler', version: '>= 0.16.0')

name = 'Musicus'
base_id = 'de.johrpan.Musicus'
metadata_url = 'https://musicus.johrpan.de/musicus_metadata_latest.musdb'
library_url = 'https://musicus.johrpan.de/musicus_library_latest.muslib'
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
  gtk_update_icon_cache: host_machine.system() != 'windows',
  update_desktop_database: host_machine.system() != 'windows',
)
