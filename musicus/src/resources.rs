use anyhow::Result;

pub fn init() -> Result<()> {
    let bytes = glib::Bytes::from(include_bytes!("/home/johrpan/Entwicklung/musicus/build/musicus/res/musicus.gresource").as_ref());
    let resource = gio::Resource::from_data(&bytes)?;
    gio::resources_register(&resource);

    Ok(())
}
