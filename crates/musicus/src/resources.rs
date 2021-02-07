use anyhow::Result;

pub fn init() -> Result<()> {
    let bytes = glib::Bytes::from(include_bytes!("/home/johrpan/.var/app/org.gnome.Builder/cache/gnome-builder/projects/musicus/builds/de.johrpan.musicus.json-flatpak-org.gnome.Platform-x86_64-master-master/crates/musicus/res/musicus.gresource").as_ref());
    let resource = gio::Resource::from_data(&bytes)?;
    gio::resources_register(&resource);

    Ok(())
}
