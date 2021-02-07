use crate::{Backend, Error, Result};
use musicus_client::LoginData;
use futures_channel::oneshot;
use secret_service::{Collection, EncryptionType, SecretService};
use std::collections::HashMap;
use std::thread;

impl Backend {
    /// Get the login credentials from secret storage.
    pub(super) async fn load_login_data() -> Result<Option<LoginData>> {
        let (sender, receiver) = oneshot::channel();
        thread::spawn(move || sender.send(Self::load_login_data_priv()).unwrap());
        receiver.await?
    }

    /// Savely store the user's current login credentials.
    pub(super) async fn store_login_data(data: LoginData) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        thread::spawn(move || sender.send(Self::store_login_data_priv(data)).unwrap());
        receiver.await?
    }

    /// Delete all stored secrets.
    pub(super) async fn delete_secrets() -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        thread::spawn(move || sender.send(Self::delete_secrets_priv()).unwrap());
        receiver.await?
    }

    /// Get the login credentials from secret storage.
    fn load_login_data_priv() -> Result<Option<LoginData>> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = Self::get_collection(&ss)?;

        let items = collection.get_all_items()?;

        let key = "musicus-login-data";
        let item = items.iter().find(|item| item.get_label().unwrap_or_default() == key);

        Ok(match item {
            Some(item) => {
                let username = item
                    .get_attributes()?
                    .get("username")
                    .ok_or(Error::Other("Missing username in SecretService attributes."))?
                    .to_owned();

                let password = std::str::from_utf8(&item.get_secret()?)?.to_owned();

                Some(LoginData { username, password })
            }
            None => None,
        })
    }

    /// Savely store the user's current login credentials.
    fn store_login_data_priv(data: LoginData) -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = Self::get_collection(&ss)?;

        let key = "musicus-login-data";
        Self::delete_secrets_for_key(&collection, key)?;

        let mut attributes = HashMap::new();
        attributes.insert("username", data.username.as_str());
        collection.create_item(key, attributes, data.password.as_bytes(), true, "text/plain")?;

        Ok(())
    }

    /// Delete all stored secrets.
    fn delete_secrets_priv() -> Result<()> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let collection = Self::get_collection(&ss)?;

        let key = "musicus-login-data";
        Self::delete_secrets_for_key(&collection, key)?;

        Ok(())
    }

    /// Delete all stored secrets for the provided key.
    fn delete_secrets_for_key(collection: &Collection, key: &str) -> Result<()> {
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
}
