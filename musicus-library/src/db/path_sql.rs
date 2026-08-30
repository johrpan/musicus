//! Diesel serialization of the supported [`LibraryTrackPath`] value.
//!
//! Kept in the private database layer so the supported API never mentions
//! storage types. The `AsExpression`/`FromSqlRow` derives live on the value
//! type itself; the actual (de)serialization is implemented here.

use anyhow::anyhow;
use diesel::{
    backend::Backend,
    deserialize::FromSql,
    serialize::{IsNull, Output, ToSql},
    sql_types::Text,
    sqlite::Sqlite,
};

use crate::path::LibraryTrackPath;

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

        // Legacy rows store a serialized JSON array of host path components
        // (e.g. `["recording_00.mp3"]`) instead of the portable form. Convert
        // them in memory; the schema migration rewrites the stored values.
        if text.trim_start().starts_with('[') {
            let components = serde_json::from_str::<Vec<String>>(&text)?;
            let joined = components.join("/");

            return LibraryTrackPath::try_from_str(&joined).map_err(|err| anyhow!(err).into());
        }

        LibraryTrackPath::try_from_str(&text).map_err(|err| anyhow!(err).into())
    }
}
