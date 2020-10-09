use super::tables::*;
use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct WorkPartDescription {
    pub title: String,
    pub composer: Option<Person>,
    pub instruments: Vec<Instrument>,
}

#[derive(Debug, Clone)]
pub struct WorkSectionDescription {
    pub title: String,
    pub before_index: i64,
}

#[derive(Debug, Clone)]
pub struct WorkDescription {
    pub id: i64,
    pub title: String,
    pub composer: Person,
    pub instruments: Vec<Instrument>,
    pub parts: Vec<WorkPartDescription>,
    pub sections: Vec<WorkSectionDescription>,
}

#[derive(Debug, Clone)]
pub struct WorkPartInsertion {
    pub part: WorkPart,
    pub instrument_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct WorkInsertion {
    pub work: Work,
    pub instrument_ids: Vec<i64>,
    pub parts: Vec<WorkPartInsertion>,
    pub sections: Vec<WorkSection>,
}

impl From<WorkDescription> for WorkInsertion {
    fn from(description: WorkDescription) -> Self {
        WorkInsertion {
            work: Work {
                id: description.id,
                composer: description.composer.id,
                title: description.title.clone(),
            },
            instrument_ids: description
                .instruments
                .iter()
                .map(|instrument| instrument.id)
                .collect(),
            parts: description
                .parts
                .iter()
                .enumerate()
                .map(|(index, part)| WorkPartInsertion {
                    part: WorkPart {
                        id: rand::random(),
                        work: description.id,
                        part_index: index.try_into().expect("Part index didn't fit into u32!"),
                        composer: part.composer.as_ref().map(|person| person.id),
                        title: part.title.clone(),
                    },
                    instrument_ids: part
                        .instruments
                        .iter()
                        .map(|instrument| instrument.id)
                        .collect(),
                })
                .collect(),
            sections: description
                .sections
                .iter()
                .map(|section| WorkSection {
                    id: rand::random(),
                    work: description.id,
                    title: section.title.clone(),
                    before_index: section.before_index,
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceDescription {
    pub person: Option<Person>,
    pub ensemble: Option<Ensemble>,
    pub role: Option<Instrument>,
}

impl PerformanceDescription {
    pub fn get_title(&self) -> String {
        let mut text = String::from(if self.is_person() {
            self.unwrap_person().name_fl()
        } else {
            self.unwrap_ensemble().name
        });

        if self.has_role() {
            text = text + " (" + &self.unwrap_role().name + ")";
        }

        text
    }

    pub fn is_person(&self) -> bool {
        self.person.is_some()
    }

    pub fn unwrap_person(&self) -> Person {
        self.person.clone().unwrap()
    }

    pub fn unwrap_ensemble(&self) -> Ensemble {
        self.ensemble.clone().unwrap()
    }

    pub fn has_role(&self) -> bool {
        self.role.clone().is_some()
    }

    pub fn unwrap_role(&self) -> Instrument {
        self.role.clone().unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct RecordingDescription {
    pub id: i64,
    pub work: WorkDescription,
    pub comment: String,
    pub performances: Vec<PerformanceDescription>,
}

#[derive(Debug, Clone)]
pub struct RecordingInsertion {
    pub recording: Recording,
    pub performances: Vec<Performance>,
}

impl From<RecordingDescription> for RecordingInsertion {
    fn from(description: RecordingDescription) -> Self {
        RecordingInsertion {
            recording: Recording {
                id: description.id,
                work: description.work.id,
                comment: description.comment.clone(),
            },
            performances: description
                .performances
                .iter()
                .map(|performance| Performance {
                    id: rand::random(),
                    recording: description.id,
                    person: performance.person.as_ref().map(|person| person.id),
                    ensemble: performance.ensemble.as_ref().map(|ensemble| ensemble.id),
                    role: performance.role.as_ref().map(|role| role.id),
                })
                .collect(),
        }
    }
}
