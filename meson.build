project(
  'musicus',
  'rust', 
  version: '0.1.0',
  meson_version: '>= 0.62.0',
)

i18n = import('i18n')
gnome = import('gnome')

# TODO: Add dependency checks for required shared libraries like SQLite,
# GStreamer (including plugins), Gtk, Libadwaita etc.

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
