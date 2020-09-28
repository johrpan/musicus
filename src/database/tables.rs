use super::schema::*;
use diesel::Queryable;

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Person {
    pub id: i64,
    pub first_name: String,
    pub last_name: String,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Instrument {
    pub id: i64,
    pub name: String,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Work {
    pub id: i64,
    pub composer: i64,
    pub title: String,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Instrumentation {
    pub id: i64,
    pub work: i64,
    pub instrument: i64,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct WorkPart {
    pub id: i64,
    pub work: i64,
    pub part_index: i64,
    pub composer: Option<i64>,
    pub title: String,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct PartInstrumentation {
    pub id: i64,
    pub work_part: i64,
    pub instrument: i64,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct WorkSection {
    pub id: i64,
    pub work: i64,
    pub title: String,
    pub before_index: i64,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Ensemble {
    pub id: i64,
    pub name: String,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Recording {
    pub id: i64,
    pub work: i64,
    pub comment: String,
}

#[derive(Insertable, Queryable, Debug, Clone)]
pub struct Performance {
    pub id: i64,
    pub recording: i64,
    pub person: Option<i64>,
    pub ensemble: Option<i64>,
    pub role: Option<i64>,
}
