use super::models::*;
use super::schema::*;
use super::tables::*;
use diesel::prelude::*;

embed_migrations!();

pub struct Database {
    c: SqliteConnection,
}

impl Database {
    pub fn new(path: &str) -> Database {
        let c = SqliteConnection::establish(path)
            .expect(&format!("Failed to connect to database at \"{}\"!", path));

        diesel::sql_query("PRAGMA foreign_keys = ON;")
            .execute(&c)
            .expect("Failed to activate foreign key support!");

        embedded_migrations::run(&c).expect("Failed to run migrations!");

        Database { c: c }
    }

    pub fn update_person(&self, person: Person) {
        diesel::replace_into(persons::table)
            .values(person)
            .execute(&self.c)
            .expect("Failed to insert person!");
    }

    pub fn get_person(&self, id: i64) -> Option<Person> {
        persons::table
            .filter(persons::id.eq(id))
            .load::<Person>(&self.c)
            .expect("Error loading person!")
            .first()
            .cloned()
    }

    pub fn delete_person(&self, id: i64) {
        diesel::delete(persons::table.filter(persons::id.eq(id)))
            .execute(&self.c)
            .expect("Failed to delete person!");
    }

    pub fn get_persons(&self) -> Vec<Person> {
        persons::table
            .load::<Person>(&self.c)
            .expect("Error loading persons!")
    }

    pub fn update_instrument(&self, instrument: Instrument) {
        diesel::replace_into(instruments::table)
            .values(instrument)
            .execute(&self.c)
            .expect("Failed to insert instrument!");
    }

    pub fn get_instrument(&self, id: i64) -> Option<Instrument> {
        instruments::table
            .filter(instruments::id.eq(id))
            .load::<Instrument>(&self.c)
            .expect("Error loading instrument!")
            .first()
            .cloned()
    }

    pub fn delete_instrument(&self, id: i64) {
        diesel::delete(instruments::table.filter(instruments::id.eq(id)))
            .execute(&self.c)
            .expect("Failed to delete instrument!");
    }

    pub fn get_instruments(&self) -> Vec<Instrument> {
        instruments::table
            .load::<Instrument>(&self.c)
            .expect("Error loading instruments!")
    }

    pub fn update_work(&self, work_insertion: WorkInsertion) {
        let id = work_insertion.work.id;

        self.delete_work(id);

        diesel::insert_into(works::table)
            .values(work_insertion.work)
            .execute(&self.c)
            .expect("Failed to insert work!");

        for instrument_id in work_insertion.instrument_ids {
            diesel::insert_into(instrumentations::table)
                .values(Instrumentation {
                    id: rand::random(),
                    work: id,
                    instrument: instrument_id,
                })
                .execute(&self.c)
                .expect("Failed to insert instrumentation!");
        }

        for part_insertion in work_insertion.parts {
            let part_id = part_insertion.part.id;

            diesel::insert_into(work_parts::table)
                .values(part_insertion.part)
                .execute(&self.c)
                .expect("Failed to insert work part!");

            for instrument_id in part_insertion.instrument_ids {
                diesel::insert_into(part_instrumentations::table)
                    .values(PartInstrumentation {
                        id: rand::random(),
                        work_part: part_id,
                        instrument: instrument_id,
                    })
                    .execute(&self.c)
                    .expect("Failed to insert part instrumentation!");
            }
        }

        for section in work_insertion.sections {
            diesel::insert_into(work_sections::table)
                .values(section)
                .execute(&self.c)
                .expect("Failed to insert work section!");
        }
    }

    pub fn get_work(&self, id: i64) -> Option<Work> {
        works::table
            .filter(works::id.eq(id))
            .load::<Work>(&self.c)
            .expect("Error loading work!")
            .first()
            .cloned()
    }

    pub fn get_work_description_for_work(&self, work: &Work) -> WorkDescription {
        WorkDescription {
            id: work.id,
            composer: self
                .get_person(work.composer)
                .expect("Could not find composer for work!"),
            title: work.title.clone(),
            instruments: instrumentations::table
                .filter(instrumentations::work.eq(work.id))
                .load::<Instrumentation>(&self.c)
                .expect("Failed to load instrumentations!")
                .iter()
                .map(|instrumentation| {
                    self.get_instrument(instrumentation.instrument)
                        .expect("Could not find instrument for instrumentation!")
                })
                .collect(),
            parts: work_parts::table
                .filter(work_parts::work.eq(work.id))
                .load::<WorkPart>(&self.c)
                .expect("Failed to load work parts!")
                .iter()
                .map(|work_part| WorkPartDescription {
                    composer: match work_part.composer {
                        Some(composer) => Some(
                            self.get_person(composer)
                                .expect("Could not find composer for work part!"),
                        ),
                        None => None,
                    },
                    title: work_part.title.clone(),
                    instruments: part_instrumentations::table
                        .filter(part_instrumentations::work_part.eq(work_part.id))
                        .load::<PartInstrumentation>(&self.c)
                        .expect("Failed to load part instrumentations!")
                        .iter()
                        .map(|part_instrumentation| {
                            self.get_instrument(part_instrumentation.instrument)
                                .expect("Could not find instrument for part instrumentation!")
                        })
                        .collect(),
                })
                .collect(),
            sections: work_sections::table
                .filter(work_sections::work.eq(work.id))
                .load::<WorkSection>(&self.c)
                .expect("Failed to load work sections!")
                .iter()
                .map(|section| WorkSectionDescription {
                    title: section.title.clone(),
                    before_index: section.before_index,
                })
                .collect(),
        }
    }

    pub fn get_work_description(&self, id: i64) -> Option<WorkDescription> {
        match self.get_work(id) {
            Some(work) => Some(self.get_work_description_for_work(&work)),
            None => None,
        }
    }

    pub fn delete_work(&self, id: i64) {
        diesel::delete(works::table.filter(works::id.eq(id)))
            .execute(&self.c)
            .expect("Failed to delete work!");
    }

    pub fn get_works(&self, composer_id: i64) -> Vec<Work> {
        works::table
            .filter(works::composer.eq(composer_id))
            .load::<Work>(&self.c)
            .expect("Error loading works!")
    }

    pub fn get_work_descriptions(&self, composer_id: i64) -> Vec<WorkDescription> {
        self.get_works(composer_id)
            .iter()
            .map(|work| self.get_work_description_for_work(work))
            .collect()
    }

    pub fn update_ensemble(&self, ensemble: Ensemble) {
        diesel::replace_into(ensembles::table)
            .values(ensemble)
            .execute(&self.c)
            .expect("Failed to insert ensemble!");
    }

    pub fn get_ensemble(&self, id: i64) -> Option<Ensemble> {
        ensembles::table
            .filter(ensembles::id.eq(id))
            .load::<Ensemble>(&self.c)
            .expect("Error loading ensemble!")
            .first()
            .cloned()
    }

    pub fn delete_ensemble(&self, id: i64) {
        diesel::delete(ensembles::table.filter(ensembles::id.eq(id)))
            .execute(&self.c)
            .expect("Failed to delete ensemble!");
    }

    pub fn get_ensembles(&self) -> Vec<Ensemble> {
        ensembles::table
            .load::<Ensemble>(&self.c)
            .expect("Error loading ensembles!")
    }

    pub fn update_recording(&self, recording_insertion: RecordingInsertion) {
        let id = recording_insertion.recording.id;

        self.delete_recording(id);

        diesel::insert_into(recordings::table)
            .values(recording_insertion.recording)
            .execute(&self.c)
            .expect("Failed to insert recording!");

        for performance in recording_insertion.performances {
            diesel::insert_into(performances::table)
                .values(performance)
                .execute(&self.c)
                .expect("Failed to insert performance!");
        }
    }

    pub fn get_recording(&self, id: i64) -> Option<Recording> {
        recordings::table
            .filter(recordings::id.eq(id))
            .load::<Recording>(&self.c)
            .expect("Error loading recording!")
            .first()
            .cloned()
    }

    pub fn get_recording_description_for_recording(
        &self,
        recording: Recording,
    ) -> RecordingDescription {
        RecordingDescription {
            id: recording.id,
            work: self
                .get_work_description(recording.work)
                .expect("Could not find work for recording!"),
            comment: recording.comment,
            performances: performances::table
                .filter(performances::recording.eq(recording.id))
                .load::<Performance>(&self.c)
                .expect("Failed to load performances!")
                .iter()
                .map(|performance| PerformanceDescription {
                    person: performance.person.map(|id| {
                        self.get_person(id)
                            .expect("Could not find person for performance!")
                    }),
                    ensemble: performance.ensemble.map(|id| {
                        self.get_ensemble(id)
                            .expect("Could not find ensemble for performance!")
                    }),
                    role: performance.role.map(|id| {
                        self.get_instrument(id)
                            .expect("Could not find role for performance!")
                    }),
                })
                .collect(),
        }
    }

    pub fn get_recording_description(&self, id: i64) -> Option<RecordingDescription> {
        match self.get_recording(id) {
            Some(recording) => Some(self.get_recording_description_for_recording(recording)),
            None => None,
        }
    }

    pub fn delete_recording(&self, id: i64) {
        diesel::delete(recordings::table.filter(recordings::id.eq(id)))
            .execute(&self.c)
            .expect("Failed to delete recording!");
    }

    pub fn get_recordings(&self, work_id: i64) -> Vec<Recording> {
        recordings::table
            .filter(recordings::work.eq(work_id))
            .load::<Recording>(&self.c)
            .expect("Error loading recordings!")
    }
}
