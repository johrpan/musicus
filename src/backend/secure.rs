use super::LoginData;
use anyhow::{anyhow, Result};
use futures_channel::oneshot;
use secret_service::{Collection, EncryptionType, SecretService};

/// Savely store the user's current login credentials.
pub async fn store_login_data(data: LoginData) -> Result<()> {
    let (sender, receiver) = oneshot::channel::<Result<()>>();
    std::thread::spawn(move || sender.send(store_login_data_priv(data)));
    receiver.await?
}

/// Savely store the user's current login credentials.
fn store_login_data_priv(data: LoginData) -> Result<()> {
    let ss = get_ss()?;
    let collection = get_collection(&ss)?;

    let key = "musicus-login-data";
    delete_secrets(&collection, key)?;

    collection
        .create_item(
            key,
            vec![("username", &data.username)],
            data.password.as_bytes(),
            true,
            "text/plain",
        )
        .or(Err(anyhow!(
            "Failed to save login data using SecretService!"
        )))?;

    Ok(())
}

/// Get the login credentials from secret storage.
pub fn load_login_data() -> Result<Option<LoginData>> {
    let ss = get_ss()?;
    let collection = get_collection(&ss)?;

    let items = collection.get_all_items().or(Err(anyhow!(
        "Failed to get items from SecretService collection!"
    )))?;

    let key = "musicus-login-data";
    let item = items
        .iter()
        .find(|item| item.get_label().unwrap_or_default() == key);

    Ok(match item {
        Some(item) => {
            let attrs = item.get_attributes().or(Err(anyhow!(
                "Failed to get attributes for ScretService item!"
            )))?;

            let username = attrs
                .iter()
                .find(|attr| attr.0 == "username")
                .ok_or(anyhow!("No username in login data!"))?
                .1
                .clone();

            let password = std::str::from_utf8(
                &item
                    .get_secret()
                    .or(Err(anyhow!("Failed to get secret from SecretService!")))?,
            )?
            .to_string();

            Some(LoginData { username, password })
        }
        None => None,
    })
}

/// Delete all stored secrets for the provided key.
fn delete_secrets(collection: &Collection, key: &str) -> Result<()> {
    let items = collection.get_all_items().or(Err(anyhow!(
        "Failed to get items from SecretService collection!"
    )))?;

    for item in items {
        if item.get_label().unwrap_or_default() == key {
            item.delete()
                .or(Err(anyhow!("Failed to delete SecretService item!")))?;
        }
    }

    Ok(())
}

/// Get the SecretService interface.
fn get_ss() -> Result<SecretService> {
    SecretService::new(EncryptionType::Dh).or(Err(anyhow!("Failed to get SecretService!")))
}

/// Get the default SecretService collection and unlock it.
fn get_collection(ss: &SecretService) -> Result<Collection> {
    let collection = ss
        .get_default_collection()
        .or(Err(anyhow!("Failed to get SecretService connection!")))?;

    collection
        .unlock()
        .or(Err(anyhow!("Failed to unclock SecretService collection!")))?;

    Ok(collection)
}
