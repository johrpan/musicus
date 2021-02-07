use super::generate_id;
use super::schema::{instrumentations, work_parts, work_sections, works};
use super::{Database, Error, Instrument, Person, Result};
use diesel::prelude::*;
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Serialize};

/// Table row data for a work.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "works"]
struct WorkRow {
    pub id: String,
    pub composer: String,
    pub title: String,
}

impl From<Work> for WorkRow {
    fn from(work: Work) -> Self {
        WorkRow {
            id: work.id,
            composer: work.composer.id,
            title: work.title,
        }
    }
}

/// Definition that a work uses an instrument.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "instrumentations"]
struct InstrumentationRow {
    pub id: i64,
    pub work: String,
    pub instrument: String,
}

/// Table row data for a work part.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "work_parts"]
struct WorkPartRow {
    pub id: i64,
    pub work: String,
    pub part_index: i64,
    pub title: String,
}

/// Table row data for a work section.
#[derive(Insertable, Queryable, Debug, Clone)]
#[table_name = "work_sections"]
struct WorkSectionRow {
    pub id: i64,
    pub work: String,
    pub title: String,
    pub before_index: i64,
}
/// A concrete work part that can be recorded.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkPart {
    pub title: String,
}

/// A heading between work parts.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorkSection {
    pub title: String,
    pub before_index: usize,
}

/// A specific work by a composer.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub id: String,
    pub title: String,
    pub composer: Person,
    pub instruments: Vec<Instrument>,
    pub parts: Vec<WorkPart>,
    pub sections: Vec<WorkSection>,
}

impl Work {
    /// Initialize a new work with a composer.
    pub fn new(composer: Person) -> Self {
        Self {
            id: generate_id(),
            title: String::new(),
            composer,
            instruments: Vec::new(),
            parts: Vec::new(),
            sections: Vec::new(),
        }
    }

    /// Get a string including the composer and title of the work.
    // TODO: Replace with impl Display.
    pub fn get_title(&self) -> String {
        format!("{}: {}", self.composer.name_fl(), self.title)
    }
}

impl Database {
    /// Update an existing work or insert a new one.
    // TODO: Think about also inserting related items.
    pub fn update_work(&self, work: Work) -> Result<()> {
        self.defer_foreign_keys()?;

        self.connection.transaction::<(), Error, _>(|| {
            let work_id = &work.id;
            self.delete_work(work_id)?;

            // Add associated items from the server, if they don't already exist.

            if self.get_person(&work.composer.id)?.is_none() {
                self.update_person(work.composer.clone())?;
            }

            for instrument in &work.instruments {
                if self.get_instrument(&instrument.id)?.is_none() {
                    self.update_instrument(instrument.clone())?;
                }
            }

            // Add the actual work.

            let row: WorkRow = work.clone().into();
            diesel::insert_into(works::table)
                .values(row)
                .execute(&self.connection)?;

            match work {
                Work {
                    instruments,
                    parts,
                    sections,
                    ..
                } => {
                    for instrument in instruments {
                        let row = InstrumentationRow {
                            id: rand::random(),
                            work: work_id.to_string(),
                            instrument: instrument.id,
                        };

                        diesel::insert_into(instrumentations::table)
                            .values(row)
                            .execute(&self.connection)?;
                    }

                    for (index, part) in parts.into_iter().enumerate() {
                        let row = WorkPartRow {
                            id: rand::random(),
                            work: work_id.to_string(),
                            part_index: index as i64,
                            title: part.title,
                        };

                        diesel::insert_into(work_parts::table)
                            .values(row)
                            .execute(&self.connection)?;
                    }

                    for section in sections {
                        let row = WorkSectionRow {
                            id: rand::random(),
                            work: work_id.to_string(),
                            title: section.title,
                            before_index: section.before_index as i64,
                        };

                        diesel::insert_into(work_sections::table)
                            .values(row)
                            .execute(&self.connection)?;
                    }
                }
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Get an existing work.
    pub fn get_work(&self, id: &str) -> Result<Option<Work>> {
        let row = works::table
            .filter(works::id.eq(id))
            .load::<WorkRow>(&self.connection)?
            .first()
            .cloned();

        let work = match row {
            Some(row) => Some(self.get_work_data(row)?),
            None => None,
        };

        Ok(work)
    }

    /// Retrieve all available information on a work from related tables.
    fn get_work_data(&self, row: WorkRow) -> Result<Work> {
        let mut instruments: Vec<Instrument> = Vec::new();

        let instrumentations = instrumentations::table
            .filter(instrumentations::work.eq(&row.id))
            .load::<InstrumentationRow>(&self.connection)?;

        for instrumentation in instrumentations {
            let id = &instrumentation.instrument;
            instruments.push(
                self.get_instrument(id)?
                    .ok_or(Error::Other(format!(
                        "Failed to get instrument ({}) for work ({}).",
                        id,
                        row.id,
                    )))?
            );
        }

        let mut parts: Vec<WorkPart> = Vec::new();

        let part_rows = work_parts::table
            .filter(work_parts::work.eq(&row.id))
            .load::<WorkPartRow>(&self.connection)?;

        for part_row in part_rows {
            parts.push(WorkPart {
                title: part_row.title,
            });
        }

        let mut sections: Vec<WorkSection> = Vec::new();

        let section_rows = work_sections::table
            .filter(work_sections::work.eq(&row.id))
            .load::<WorkSectionRow>(&self.connection)?;

        for section_row in section_rows {
            sections.push(WorkSection {
                title: section_row.title,
                before_index: section_row.before_index as usize,
            });
        }

        let person_id = &row.composer;
        let person = self
            .get_person(person_id)?
            .ok_or(Error::Other(format!(
                "Failed to get person ({}) for work ({}).",
                person_id,
                row.id,
            )))?;

        Ok(Work {
            id: row.id,
            composer: person,
            title: row.title,
            instruments,
            parts,
            sections,
        })
    }

    /// Delete an existing work. This will fail if there are still other tables that relate to
    /// this work except for the things that are part of the information on the work it
    pub fn delete_work(&self, id: &str) -> Result<()> {
        diesel::delete(works::table.filter(works::id.eq(id))).execute(&self.connection)?;
        Ok(())
    }

    /// Get all existing works by a composer and related information from other tables.
    pub fn get_works(&self, composer_id: &str) -> Result<Vec<Work>> {
        let mut works: Vec<Work> = Vec::new();

        let rows = works::table
            .filter(works::composer.eq(composer_id))
            .load::<WorkRow>(&self.connection)?;

        for row in rows {
            works.push(self.get_work_data(row)?);
        }

        Ok(works)
    }
}
