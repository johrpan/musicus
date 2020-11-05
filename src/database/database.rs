use super::models::*;
use super::schema::*;
use super::tables::*;
use anyhow::{anyhow, Error, Result};
use diesel::prelude::*;
use std::convert::TryInto;

embed_migrations!();

pub struct Database {
    c: SqliteConnection,
}

impl Database {
    pub fn new(path: &str) -> Result<Database> {
        let c = SqliteConnection::establish(path)?;

        diesel::sql_query("PRAGMA foreign_keys = ON;").execute(&c)?;
        embedded_migrations::run(&c)?;

        Ok(Database { c: c })
    }

    pub fn update_person(&self, person: Person) -> Result<()> {
        self.defer_foreign_keys();
        self.c.transaction(|| {
            diesel::replace_into(persons::table)
                .values(person)
                .execute(&self.c)
        })?;

        Ok(())
    }

    pub fn get_person(&self, id: i64) -> Result<Person> {
        persons::table
            .filter(persons::id.eq(id))
            .load::<Person>(&self.c)?
            .first()
            .cloned()
            .ok_or(anyhow!("No person with ID: {}", id))
    }

    pub fn delete_person(&self, id: i64) -> Result<()> {
        diesel::delete(persons::table.filter(persons::id.eq(id))).execute(&self.c)?;
        Ok(())
    }

    pub fn get_persons(&self) -> Result<Vec<Person>> {
        let persons = persons::table.load::<Person>(&self.c)?;
        Ok(persons)
    }

    pub fn update_instrument(&self, instrument: Instrument) -> Result<()> {
        self.defer_foreign_keys();
        self.c.transaction(|| {
            diesel::replace_into(instruments::table)
                .values(instrument)
                .execute(&self.c)
        })?;

        Ok(())
    }

    pub fn get_instrument(&self, id: i64) -> Result<Instrument> {
        instruments::table
            .filter(instruments::id.eq(id))
            .load::<Instrument>(&self.c)?
            .first()
            .cloned()
            .ok_or(anyhow!("No instrument with ID: {}", id))
    }

    pub fn delete_instrument(&self, id: i64) -> Result<()> {
        diesel::delete(instruments::table.filter(instruments::id.eq(id))).execute(&self.c)?;
        Ok(())
    }

    pub fn get_instruments(&self) -> Result<Vec<Instrument>> {
        let instruments = instruments::table.load::<Instrument>(&self.c)?;
        Ok(instruments)
    }

    pub fn update_work(&self, work_insertion: WorkInsertion) -> Result<()> {
        let id = work_insertion.work.id;

        self.defer_foreign_keys();
        self.c.transaction::<(), Error, _>(|| {
            self.delete_work(id)?;

            diesel::insert_into(works::table)
                .values(work_insertion.work)
                .execute(&self.c)?;

            for instrument_id in work_insertion.instrument_ids {
                diesel::insert_into(instrumentations::table)
                    .values(Instrumentation {
                        id: rand::random(),
                        work: id,
                        instrument: instrument_id,
                    })
                    .execute(&self.c)?;
            }

            for part_insertion in work_insertion.parts {
                let part_id = part_insertion.part.id;

                diesel::insert_into(work_parts::table)
                    .values(part_insertion.part)
                    .execute(&self.c)?;

                for instrument_id in part_insertion.instrument_ids {
                    diesel::insert_into(part_instrumentations::table)
                        .values(PartInstrumentation {
                            id: rand::random(),
                            work_part: part_id,
                            instrument: instrument_id,
                        })
                        .execute(&self.c)?;
                }
            }

            for section in work_insertion.sections {
                diesel::insert_into(work_sections::table)
                    .values(section)
                    .execute(&self.c)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn get_work(&self, id: i64) -> Result<Work> {
        works::table
            .filter(works::id.eq(id))
            .load::<Work>(&self.c)?
            .first()
            .cloned()
            .ok_or(anyhow!("No work with ID: {}", id))
    }

    pub fn get_work_description_for_work(&self, work: &Work) -> Result<WorkDescription> {
        let mut instruments: Vec<Instrument> = Vec::new();

        let instrumentations = instrumentations::table
            .filter(instrumentations::work.eq(work.id))
            .load::<Instrumentation>(&self.c)?;

        for instrumentation in instrumentations {
            instruments.push(self.get_instrument(instrumentation.instrument)?);
        }

        let mut part_descriptions: Vec<WorkPartDescription> = Vec::new();

        let work_parts = work_parts::table
            .filter(work_parts::work.eq(work.id))
            .load::<WorkPart>(&self.c)?;

        for work_part in work_parts {
            let mut part_instruments: Vec<Instrument> = Vec::new();

            let part_instrumentations = part_instrumentations::table
                .filter(part_instrumentations::work_part.eq(work_part.id))
                .load::<PartInstrumentation>(&self.c)?;

            for part_instrumentation in part_instrumentations {
                part_instruments.push(self.get_instrument(part_instrumentation.instrument)?);
            }

            part_descriptions.push(WorkPartDescription {
                composer: match work_part.composer {
                    Some(composer) => Some(self.get_person(composer)?),
                    None => None,
                },
                title: work_part.title.clone(),
                instruments: part_instruments,
            });
        }

        let mut section_descriptions: Vec<WorkSectionDescription> = Vec::new();

        let sections = work_sections::table
            .filter(work_sections::work.eq(work.id))
            .load::<WorkSection>(&self.c)?;

        for section in sections {
            section_descriptions.push(WorkSectionDescription {
                title: section.title.clone(),
                before_index: section.before_index,
            });
        }

        let work_description = WorkDescription {
            id: work.id,
            composer: self.get_person(work.composer)?,
            title: work.title.clone(),
            instruments: instruments,
            parts: part_descriptions,
            sections: section_descriptions,
        };

        Ok(work_description)
    }

    pub fn get_work_description(&self, id: i64) -> Result<WorkDescription> {
        let work = self.get_work(id)?;
        let work_description = self.get_work_description_for_work(&work)?;
        Ok(work_description)
    }

    pub fn delete_work(&self, id: i64) -> Result<()> {
        diesel::delete(works::table.filter(works::id.eq(id))).execute(&self.c)?;
        Ok(())
    }

    pub fn get_works(&self, composer_id: i64) -> Result<Vec<Work>> {
        let works = works::table
            .filter(works::composer.eq(composer_id))
            .load::<Work>(&self.c)?;

        Ok(works)
    }

    pub fn get_work_descriptions(&self, composer_id: i64) -> Result<Vec<WorkDescription>> {
        let mut work_descriptions: Vec<WorkDescription> = Vec::new();

        let works = self.get_works(composer_id)?;
        for work in works {
            work_descriptions.push(self.get_work_description_for_work(&work)?);
        }

        Ok(work_descriptions)
    }

    pub fn update_ensemble(&self, ensemble: Ensemble) -> Result<()> {
        self.defer_foreign_keys();
        self.c.transaction(|| {
            diesel::replace_into(ensembles::table)
                .values(ensemble)
                .execute(&self.c)
        })?;

        Ok(())
    }

    pub fn get_ensemble(&self, id: i64) -> Result<Ensemble> {
        ensembles::table
            .filter(ensembles::id.eq(id))
            .load::<Ensemble>(&self.c)?
            .first()
            .cloned()
            .ok_or(anyhow!("No ensemble with ID: {}", id))
    }

    pub fn delete_ensemble(&self, id: i64) -> Result<()> {
        diesel::delete(ensembles::table.filter(ensembles::id.eq(id))).execute(&self.c)?;
        Ok(())
    }

    pub fn get_ensembles(&self) -> Result<Vec<Ensemble>> {
        let ensembles = ensembles::table.load::<Ensemble>(&self.c)?;
        Ok(ensembles)
    }

    pub fn update_recording(&self, recording_insertion: RecordingInsertion) -> Result<()> {
        let id = recording_insertion.recording.id;

        self.defer_foreign_keys();
        self.c.transaction::<(), Error, _>(|| {
            self.delete_recording(id)?;

            diesel::insert_into(recordings::table)
                .values(recording_insertion.recording)
                .execute(&self.c)?;

            for performance in recording_insertion.performances {
                diesel::insert_into(performances::table)
                    .values(performance)
                    .execute(&self.c)?;
            }

            Ok(())
        })?;

        Ok(())
    }

    pub fn get_recording(&self, id: i64) -> Result<Recording> {
        recordings::table
            .filter(recordings::id.eq(id))
            .load::<Recording>(&self.c)?
            .first()
            .cloned()
            .ok_or(anyhow!("No recording with ID: {}", id))
    }

    pub fn get_recording_description_for_recording(
        &self,
        recording: &Recording,
    ) -> Result<RecordingDescription> {
        let mut performance_descriptions: Vec<PerformanceDescription> = Vec::new();

        let performances = performances::table
            .filter(performances::recording.eq(recording.id))
            .load::<Performance>(&self.c)?;

        for performance in performances {
            performance_descriptions.push(PerformanceDescription {
                person: match performance.person {
                    Some(id) => Some(self.get_person(id)?),
                    None => None,
                },
                ensemble: match performance.ensemble {
                    Some(id) => Some(self.get_ensemble(id)?),
                    None => None,
                },
                role: match performance.role {
                    Some(id) => Some(self.get_instrument(id)?),
                    None => None,
                },
            });
        }

        Ok(RecordingDescription {
            id: recording.id,
            work: self.get_work_description(recording.work)?,
            comment: recording.comment.clone(),
            performances: performance_descriptions,
        })
    }

    pub fn get_recording_description(&self, id: i64) -> Result<RecordingDescription> {
        let recording = self.get_recording(id)?;
        let recording_description = self.get_recording_description_for_recording(&recording)?;
        Ok(recording_description)
    }

    pub fn get_recordings_for_person(&self, id: i64) -> Result<Vec<RecordingDescription>> {
        let mut recording_descriptions: Vec<RecordingDescription> = Vec::new();

        let recordings = recordings::table
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(persons::table.on(persons::id.nullable().eq(performances::person)))
            .filter(persons::id.eq(id))
            .select(recordings::table::all_columns())
            .load::<Recording>(&self.c)?;

        for recording in recordings {
            recording_descriptions.push(self.get_recording_description_for_recording(&recording)?);
        }

        Ok(recording_descriptions)
    }

    pub fn get_recordings_for_ensemble(&self, id: i64) -> Result<Vec<RecordingDescription>> {
        let mut recording_descriptions: Vec<RecordingDescription> = Vec::new();

        let recordings = recordings::table
            .inner_join(performances::table.on(performances::recording.eq(recordings::id)))
            .inner_join(ensembles::table.on(ensembles::id.nullable().eq(performances::ensemble)))
            .filter(ensembles::id.eq(id))
            .select(recordings::table::all_columns())
            .load::<Recording>(&self.c)?;

        for recording in recordings {
            recording_descriptions.push(self.get_recording_description_for_recording(&recording)?);
        }

        Ok(recording_descriptions)
    }

    pub fn get_recordings_for_work(&self, id: i64) -> Result<Vec<RecordingDescription>> {
        let mut recording_descriptions: Vec<RecordingDescription> = Vec::new();

        let recordings = recordings::table
            .inner_join(works::table.on(works::id.eq(recordings::work)))
            .filter(works::id.eq(id))
            .select(recordings::table::all_columns())
            .load::<Recording>(&self.c)?;

        for recording in recordings {
            recording_descriptions.push(self.get_recording_description_for_recording(&recording)?);
        }

        Ok(recording_descriptions)
    }

    pub fn delete_recording(&self, id: i64) -> Result<()> {
        diesel::delete(recordings::table.filter(recordings::id.eq(id))).execute(&self.c)?;
        Ok(())
    }

    pub fn get_recordings(&self, work_id: i64) -> Result<Vec<Recording>> {
        let recordings = recordings::table
            .filter(recordings::work.eq(work_id))
            .load::<Recording>(&self.c)?;

        Ok(recordings)
    }

    pub fn update_tracks(&self, recording_id: i64, tracks: Vec<TrackDescription>) -> Result<()> {
        self.delete_tracks(recording_id)?;

        for (index, track_description) in tracks.iter().enumerate() {
            let track = Track {
                id: rand::random(),
                file_name: track_description.file_name.clone(),
                recording: recording_id,
                track_index: index.try_into().unwrap(),
                work_parts: track_description
                    .work_parts
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            };

            diesel::insert_into(tracks::table)
                .values(track)
                .execute(&self.c)?;
        }

        Ok(())
    }

    pub fn delete_tracks(&self, recording_id: i64) -> Result<()> {
        diesel::delete(tracks::table.filter(tracks::recording.eq(recording_id))).execute(&self.c)?;
        Ok(())
    }

    pub fn get_tracks(&self, recording_id: i64) -> Result<Vec<TrackDescription>> {
        let tracks = tracks::table
            .filter(tracks::recording.eq(recording_id))
            .order_by(tracks::track_index)
            .load::<Track>(&self.c)?;

        Ok(tracks.iter().map(|track| track.clone().into()).collect())
    }

    fn defer_foreign_keys(&self) {
        diesel::sql_query("PRAGMA defer_foreign_keys = ON;")
            .execute(&self.c)
            .expect("Failed to enable defer_foreign_keys_pragma!");
    }
}
