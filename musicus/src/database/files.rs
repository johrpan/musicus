use super::schema::files;
use super::Database;
use anyhow::Result;
use diesel::prelude::*;

/// Table data to associate audio files with tracks.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "files"]
struct FileRow {
    pub file_name: String,
    pub track: String,
}

impl Database {
    /// Insert or update a file. This assumes that the track is already in the
    /// database.
    pub fn update_file(&self, file_name: &str, track_id: &str) -> Result<()> {
        let row = FileRow {
            file_name: file_name.to_owned(),
            track: track_id.to_owned(),
        };

        diesel::insert_into(files::table)
            .values(row)
            .execute(&self.connection)?;

        Ok(())
    }

    /// Delete an existing file. This will not delete the file from the file
    /// system but just the representing row from the database.
    pub fn delete_file(&self, file_name: &str) -> Result<()> {
        diesel::delete(files::table.filter(files::file_name.eq(file_name)))
            .execute(&self.connection)?;

        Ok(())
    }

    /// Get the file name of the audio file for the specified track.
    pub fn get_file(&self, track_id: &str) -> Result<Option<String>> {
        let row = files::table
            .filter(files::track.eq(track_id))
            .load::<FileRow>(&self.connection)?
            .into_iter()
            .next();

        let file_name = match row {
            Some(row) => Some(row.file_name),
            None => None,
        };

        Ok(file_name)
    }
}
