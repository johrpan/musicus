use super::schema::users;
use super::DbConn;
use anyhow::Result;
use diesel::prelude::*;
use serde::Deserialize;

/// A user that can be authenticated to use the API.
#[derive(Insertable, Queryable, Debug, Clone)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub email: Option<String>,
    pub is_admin: bool,
    pub is_editor: bool,
    pub is_banned: bool,
}

impl User {
    /// Check whether the user is allowed to create a new item.
    pub fn may_create(&self) -> bool {
        !self.is_banned
    }

    /// Check whether the user is allowed to edit an item created by him or somebody else.
    pub fn may_edit(&self, creator: &str) -> bool {
        !self.is_banned && (self.username == creator || self.is_editor)
    }

    /// Check whether the user is allowed to delete an item.
    pub fn may_delete(&self) -> bool {
        !self.is_banned && self.is_editor
    }
}

/// A structure representing data on a user.
#[derive(AsChangeset, Deserialize, Debug, Clone)]
#[table_name = "users"]
#[serde(rename_all = "camelCase")]
pub struct UserInsertion {
    pub password_hash: String,
    pub email: Option<String>,
}

/// Insert a new user.
pub fn insert_user(conn: &DbConn, username: &str, data: &UserInsertion) -> Result<()> {
    let user = User {
        username: username.to_string(),
        password_hash: data.password_hash.clone(),
        email: data.email.clone(),
        is_admin: false,
        is_editor: false,
        is_banned: false,
    };
    diesel::insert_into(users::table)
        .values(user)
        .execute(conn)?;

    Ok(())
}

/// Update an existing user.
pub fn update_user(conn: &DbConn, username: &str, data: &UserInsertion) -> Result<()> {
    diesel::update(users::table)
        .filter(users::username.eq(username))
        .set(data)
        .execute(conn)?;

    Ok(())
}

/// Get an existing user.
pub fn get_user(conn: &DbConn, username: &str) -> Result<Option<User>> {
    Ok(users::table
        .filter(users::username.eq(username))
        .load::<User>(conn)?
        .first()
        .cloned())
}
