use super::ServerError;
use crate::database;
use crate::database::{DbConn, DbPool, User, UserInsertion};
use actix_web::{get, post, put, web, HttpResponse};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::pwhash::argon2id13;

/// Request body data for user registration.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserRegistration {
    pub username: String,
    pub password: String,
    pub email: Option<String>,
}

/// Request body data for user login.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Login {
    pub username: String,
    pub password: String,
}

/// Request body data for changing user details.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PutUser {
    pub old_password: String,
    pub new_password: Option<String>,
    pub email: Option<String>,
}

/// Response body data for getting a user.
#[derive(Serialize, Debug, Clone)]
pub struct GetUser {
    pub username: String,
    pub email: Option<String>,
}

/// Claims for issued JWTs.
#[derive(Deserialize, Serialize, Debug, Clone)]
struct Claims {
    pub iat: u64,
    pub exp: u64,
    pub username: String,
}

/// Register a new user.
#[post("/users")]
pub async fn register_user(
    db: web::Data<DbPool>,
    data: web::Json<UserRegistration>,
) -> Result<HttpResponse, ServerError> {
    web::block(move || {
        let conn = db.into_inner().get().or(Err(ServerError::Internal))?;

        database::insert_user(
            &conn,
            &data.username,
            &UserInsertion {
                password_hash: hash_password(&data.password).or(Err(ServerError::Internal))?,
                email: data.email.clone(),
            },
        )
        .or(Err(ServerError::Internal))
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

/// Update an existing user. This doesn't use a JWT for authentication but requires the client to
/// resent the old password.
#[put("/users/{username}")]
pub async fn put_user(
    db: web::Data<DbPool>,
    username: web::Path<String>,
    data: web::Json<PutUser>,
) -> Result<HttpResponse, ServerError> {
    let conn = db.into_inner().get().or(Err(ServerError::Internal))?;

    web::block(move || {
        let user = database::get_user(&conn, &username)
            .or(Err(ServerError::Internal))?
            .ok_or(ServerError::Unauthorized)?;

        if verify_password(&data.old_password, &user.password_hash) {
            let password_hash = match &data.new_password {
                Some(password) => hash_password(password).or(Err(ServerError::Unauthorized))?,
                None => user.password_hash.clone(),
            };

            database::update_user(
                &conn,
                &username,
                &UserInsertion {
                    email: data.email.clone(),
                    password_hash,
                },
            )
            .or(Err(ServerError::Internal))?;

            Ok(())
        } else {
            Err(ServerError::Forbidden)
        }
    })
    .await?;

    Ok(HttpResponse::Ok().finish())
}

/// Get an existing user. This requires a valid JWT authenticating that user.
#[get("/users/{username}")]
pub async fn get_user(
    db: web::Data<DbPool>,
    username: web::Path<String>,
    auth: BearerAuth,
) -> Result<HttpResponse, ServerError> {
    let user = web::block(move || {
        let conn = db.into_inner().get().or(Err(ServerError::Internal))?;
        authenticate(&conn, auth.token()).or(Err(ServerError::Unauthorized))
    })
    .await?;

    if username.into_inner() != user.username {
        Err(ServerError::Forbidden)?;
    }

    Ok(HttpResponse::Ok().json(GetUser {
        username: user.username,
        email: user.email,
    }))
}

/// Login an already existing user. This will respond with a newly issued JWT.
#[post("/login")]
pub async fn login_user(
    db: web::Data<DbPool>,
    data: web::Json<Login>,
) -> Result<HttpResponse, ServerError> {
    let token = web::block(move || {
        let conn = db.into_inner().get().or(Err(ServerError::Internal))?;

        let user = database::get_user(&conn, &data.username)
            .or(Err(ServerError::Internal))?
            .ok_or(ServerError::Unauthorized)?;

        if verify_password(&data.password, &user.password_hash) {
            issue_jwt(&user.username).or(Err(ServerError::Internal))
        } else {
            Err(ServerError::Unauthorized)
        }
    })
    .await?;

    Ok(HttpResponse::Ok().body(token))
}

/// Authenticate a user by verifying the provided token. The environemtn variable "MUSICUS_SECRET"
/// will be used as the secret key and has to be set.
pub fn authenticate(conn: &DbConn, token: &str) -> Result<User> {
    let username = verify_jwt(token)?.username;
    database::get_user(conn, &username)?.ok_or(anyhow!("User doesn't exist: {}", &username))
}

/// Check whether a token allows the user to create a new item.
pub fn may_create(conn: &DbConn, token: &str) -> Result<bool> {
    let user = authenticate(conn, token)?;

    let result = if user.is_banned { false } else { false };

    Ok(result)
}

/// Check whether a token allows the user to edit an item created by him or somebody else.
pub fn may_edit(conn: &DbConn, token: &str, created_by: &str) -> Result<bool> {
    let user = authenticate(conn, token)?;

    let result = if user.is_banned {
        false
    } else if user.username == created_by {
        true
    } else if user.is_editor {
        true
    } else {
        false
    };

    Ok(result)
}

/// Return a hash for a password that can be stored in the database.
fn hash_password(password: &str) -> Result<String> {
    let hash = argon2id13::pwhash(
        password.as_bytes(),
        argon2id13::OPSLIMIT_INTERACTIVE,
        argon2id13::MEMLIMIT_INTERACTIVE,
    )
    .or(Err(anyhow!("Failed to hash password!")))?;

    // Strip trailing null bytes to facilitate database storage.
    Ok(std::str::from_utf8(&hash.0)?
        .trim_end_matches('\u{0}')
        .to_string())
}

/// Verify whether a hash is valid for a password.
fn verify_password(password: &str, hash: &str) -> bool {
    // Readd the trailing null bytes padding.
    let mut bytes = [0u8; 128];
    for (index, byte) in hash.as_bytes().iter().enumerate() {
        bytes[index] = *byte;
    }

    argon2id13::pwhash_verify(
        &argon2id13::HashedPassword::from_slice(&bytes).unwrap(),
        password.as_bytes(),
    )
}

/// Issue a JWT that allows to claim to be a user. This uses the value of the environment variable
/// "MUSICUS_SECRET" as the secret key. This needs to be set.
fn issue_jwt(username: &str) -> Result<String> {
    let now = std::time::SystemTime::now();
    let expiry = now + std::time::Duration::new(86400, 0);

    let iat = now.duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let exp = expiry.duration_since(std::time::UNIX_EPOCH)?.as_secs();

    let secret = std::env::var("MUSICUS_SECRET")?;

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &Claims {
            iat,
            exp,
            username: username.to_string(),
        },
        &jsonwebtoken::EncodingKey::from_secret(&secret.as_bytes()),
    )?;

    Ok(token)
}

/// Verify a JWT and return the claims that are made by it. This uses the value of the environment
/// variable "MUSICUS_SECRET" as the secret key. This needs to be set.
fn verify_jwt(token: &str) -> Result<Claims> {
    let secret = std::env::var("MUSICUS_SECRET")?;

    let jwt = jsonwebtoken::decode::<Claims>(
        token,
        &jsonwebtoken::DecodingKey::from_secret(&secret.as_bytes()),
        &jsonwebtoken::Validation::default(),
    )?;

    Ok(jwt.claims)
}
