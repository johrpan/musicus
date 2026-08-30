//! Diesel serialization of the supported [`LibraryTrackPath`] value.
//!
//! Kept in the private database layer so the supported API never mentions
//! storage types. The `AsExpression`/`FromSqlRow` derives live on the value
//! type itself; the actual (de)serialization is implemented here.

use anyhow::anyhow;
use diesel::{
    backend::Backend,
    deserialize::{FromSql, FromSqlRow},
    expression::AsExpression,
    serialize::{IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};

use crate::path::LibraryTrackPath;

// Generate Diesel's expression/row implementations here so the supported
// domain module does not depend on or expose persistence concepts.
#[allow(dead_code)]
#[derive(AsExpression, FromSqlRow)]
#[diesel(foreign_derive)]
#[diesel(sql_type = Text)]
struct LibraryTrackPathProxy(LibraryTrackPath);

impl ToSql<Text, Sqlite> for LibraryTrackPath
where
    String: ToSql<Text, Sqlite>,
{
    fn to_sql(&self, out: &mut Output<Sqlite>) -> diesel::serialize::Result {
        out.set_value(self.as_str().to_owned());
        Ok(IsNull::No)
    }
}

impl<DB> FromSql<Text, DB> for LibraryTrackPath
where
    DB: Backend,
    String: FromSql<Text, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let text = String::from_sql(bytes)?;

        LibraryTrackPath::try_from_str(&text).map_err(|err| anyhow!(err).into())
    }
}
