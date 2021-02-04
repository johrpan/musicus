use crate::backend::LoginData;
use anyhow::Result;
use futures_channel::oneshot;
use secret_service::{Collection, EncryptionType, SecretService};
use std::collections::HashMap;
use std::thread;

/// Savely store the user's current login credentials.
pub async fn store_login_data(data: LoginData) -> Result<()> {
    let (sender, receiver) = oneshot::channel();
    thread::spawn(move || sender.send(store_login_data_priv(data)).unwrap());
    receiver.await?
}

/// Savely store the user's current login credentials.
fn store_login_data_priv(data: LoginData) -> Result<()> {
    let ss = SecretService::new(EncryptionType::Dh)?;
    let collection = get_collection(&ss)?;

    let key = "musicus-login-data";
    delete_secrets(&collection, key)?;

    let mut attributes = HashMap::new();
    attributes.insert("username", data.username.as_str());
    collection.create_item(key, attributes, data.password.as_bytes(), true, "text/plain")?;

    Ok(())
}

/// Get the login credentials from secret storage.
pub fn load_login_data() -> Result<Option<LoginData>> {
    let ss = SecretService::new(EncryptionType::Dh)?;
    let collection = get_collection(&ss)?;

    let items = collection.get_all_items()?;

    let key = "musicus-login-data";
    let item = items.iter().find(|item| item.get_label().unwrap_or_default() == key);

    Ok(match item {
        Some(item) => {
            // TODO: Delete the item when malformed.
            let username = item.get_attributes()?.get("username").unwrap().to_owned();
            let password = std::str::from_utf8(&item.get_secret()?)?.to_owned();

            Some(LoginData { username, password })
        }
        None => None,
    })
}

/// Delete all stored secrets for the provided key.
fn delete_secrets(collection: &Collection, key: &str) -> Result<()> {
    let items = collection.get_all_items()?;

    for item in items {
        if item.get_label().unwrap_or_default() == key {
            item.delete()?;
        }
    }

    Ok(())
}

/// Get the default SecretService collection and unlock it.
fn get_collection<'a>(ss: &'a SecretService) -> Result<Collection<'a>> {
    let collection = ss.get_default_collection()?;
    collection.unlock()?;

    Ok(collection)
}
