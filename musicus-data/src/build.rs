//! Compiling the JSON metadata definitions into a metadata database.
//!
//! The result is a plain Musicus library database populated with the pre-made
//! catalogue: roles, instruments, ensembles, persons, tags and the full work
//! hierarchy. Recordings, tracks and albums only exist in user libraries, so
//! the metadata database deliberately contains none of them.
//!
//! Everything is written through `musicus-library`'s public database layer
//! (`db::connect` opens and migrates the file, the `db::schema`/`db::tables`
//! modules describe the rows), so the output is exactly what a Musicus
//! library expects as its downloaded `metadata.musdb`.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::{anyhow, bail, Context, Result};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use musicus_library::db::{
    self,
    schema::*,
    tables::{self, Source},
    TranslatedString,
};

use crate::sources::{ComposerRef, MetadataFile, TagRef, WorkEntity, WorksFile};

/// The identifier sets built while validating, plus the problems that are
/// worth reporting but not worth failing on.
#[derive(Debug)]
pub struct Validation {
    pub warnings: Vec<String>,
    pub persons: HashSet<String>,
    pub roles: HashSet<String>,
    pub instruments: HashSet<String>,
    pub ensembles: HashSet<String>,
    pub tag_takes_value: HashMap<String, bool>,
    pub work_keys: HashSet<String>,
}

/// What was written, for the summary at the end of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub persons: usize,
    pub roles: usize,
    pub instruments: usize,
    pub ensembles: usize,
    pub tags: usize,
    pub works: usize,
    pub composer_credits: usize,
    pub tag_associations: usize,
    pub instrumentations: usize,
    pub derived: usize,
}

/// The result of a build: the numbers written plus the problems found along
/// the way.
#[derive(Debug)]
pub struct BuildOutput {
    pub summary: Summary,
    pub warnings: Vec<String>,
}

/// Check both definition files against each other.
///
/// Format violations are errors. Problems the database cannot represent
/// faithfully (for example a value on a tag that does not take one) are
/// warnings.
pub fn validate(metadata: &MetadataFile, works: &WorksFile) -> Result<Validation> {
    let persons = unique_keys(&metadata.persons, "person", |entry| &entry.key)?;
    let roles = unique_keys(&metadata.roles, "role", |entry| &entry.key)?;
    let instruments = unique_keys(&metadata.instruments, "instrument", |entry| &entry.key)?;
    let ensembles = unique_keys(&metadata.ensembles, "ensemble", |entry| &entry.key)?;
    unique_keys(&metadata.tags, "tag", |entry| &entry.key)?;

    require_generic_name(
        "person",
        &metadata.persons,
        |entry| &entry.key,
        |entry| &entry.name,
    )?;
    require_generic_name(
        "role",
        &metadata.roles,
        |entry| &entry.key,
        |entry| &entry.name,
    )?;
    require_generic_name(
        "instrument",
        &metadata.instruments,
        |entry| &entry.key,
        |entry| &entry.name,
    )?;
    require_generic_name(
        "ensemble",
        &metadata.ensembles,
        |entry| &entry.key,
        |entry| &entry.name,
    )?;
    require_generic_name(
        "tag",
        &metadata.tags,
        |entry| &entry.key,
        |entry| &entry.name,
    )?;

    let tag_takes_value = metadata
        .tags
        .iter()
        .map(|tag| (tag.key.clone(), tag.takes_value))
        .collect();

    // Works form a single namespace of keys across every nesting level.
    let mut work_keys = HashSet::new();
    index_works(&works.works, &mut work_keys)?;

    // Related works must exist; the relation is meant to point from a
    // derivative towards the main version, so a cycle is a mistake. The
    // parent chain cannot cycle (the file is a tree), but the related-work
    // graph is not.
    let mut related_to: HashMap<&str, &str> = HashMap::new();
    collect_related(&works.works, &mut related_to);

    for (work_id, target) in &related_to {
        if !work_keys.contains(*target) {
            bail!("Work \"{work_id}\" relates to unknown work \"{target}\"");
        }
    }

    let mut state: HashMap<&str, u8> = HashMap::new();
    for &work_id in related_to.keys() {
        let mut stack = Vec::new();
        check_relates_cycle(work_id, &related_to, &mut state, &mut stack)?;
    }

    let mut warnings = Vec::new();
    let mut context = CheckContext {
        persons: &persons,
        roles: &roles,
        instruments: &instruments,
        tag_takes_value: &tag_takes_value,
        warnings: &mut warnings,
    };
    for work in &works.works {
        check_work(work, &mut context)?;
    }

    Ok(Validation {
        warnings,
        persons,
        roles,
        instruments,
        ensembles,
        tag_takes_value,
        work_keys,
    })
}

/// Build the metadata database at `output` from the definitions.
///
/// The output file is replaced if it exists; the database is fully derived
/// from the source files.
pub fn build(metadata: &MetadataFile, works: &WorksFile, output: &Path) -> Result<BuildOutput> {
    let validation = validate(metadata, works)?;
    let summary = write_database(metadata, works, output)?;

    Ok(BuildOutput {
        summary,
        warnings: validation.warnings,
    })
}

struct CheckContext<'a> {
    persons: &'a HashSet<String>,
    roles: &'a HashSet<String>,
    instruments: &'a HashSet<String>,
    tag_takes_value: &'a HashMap<String, bool>,
    warnings: &'a mut Vec<String>,
}

/// Collect the identifiers of a list of entries, rejecting duplicates.
fn unique_keys<'a, T>(
    items: &'a [T],
    kind: &str,
    key: impl Fn(&'a T) -> &'a str,
) -> Result<HashSet<String>> {
    let mut seen = HashSet::new();
    for item in items {
        if !seen.insert(key(item).to_owned()) {
            bail!("Duplicate {kind} key \"{}\"", key(item));
        }
    }
    Ok(seen)
}

/// Every entity needs its canonical generic name.
fn require_generic_name<'a, T>(
    kind: &str,
    list: &'a [T],
    key: impl Fn(&'a T) -> &str,
    name: impl Fn(&'a T) -> &'a TranslatedString,
) -> Result<()> {
    for entry in list {
        if name(entry)
            .0
            .get("generic")
            .is_none_or(|name| name.trim().is_empty())
        {
            bail!("{kind} \"{}\" has no generic name", key(entry));
        }
    }
    Ok(())
}

/// Index every work at every nesting level by its key.
fn index_works(list: &[WorkEntity], keys: &mut HashSet<String>) -> Result<()> {
    for work in list {
        if !keys.insert(work.key.clone()) {
            bail!("Duplicate work key \"{}\"", work.key);
        }
        index_works(&work.parts, keys)?;
    }
    Ok(())
}

/// Collect the related-work edges of every work at every nesting level.
fn collect_related<'a>(list: &'a [WorkEntity], edges: &mut HashMap<&'a str, &'a str>) {
    for work in list {
        if let Some(target) = work.related_work.as_deref() {
            edges.insert(work.key.as_str(), target);
        }
        collect_related(&work.parts, edges);
    }
}

/// Depth-first cycle detection over the related-work graph.
///
/// `state` tracks whether a node is unvisited (absent), currently on the
/// search stack (`1`) or fully explored (`2`).
fn check_relates_cycle<'a>(
    work_id: &'a str,
    related_to: &HashMap<&'a str, &'a str>,
    state: &mut HashMap<&'a str, u8>,
    stack: &mut Vec<&'a str>,
) -> Result<()> {
    match state.get(work_id).copied() {
        Some(1) => {
            let start = stack
                .iter()
                .position(|node| *node == work_id)
                .expect("a node on the stack is always in it");

            let mut cycle: Vec<&str> = stack[start..].to_vec();
            cycle.push(stack[start]);
            bail!(
                "Related works form a cycle: {}",
                cycle
                    .iter()
                    .map(|node| format!("\"{node}\""))
                    .collect::<Vec<_>>()
                    .join(" -> ")
            );
        }
        Some(2) => return Ok(()),
        _ => {}
    }

    state.insert(work_id, 1);
    stack.push(work_id);

    if let Some(&next) = related_to.get(work_id) {
        check_relates_cycle(next, related_to, state, stack)?;
    }

    stack.pop();
    state.insert(work_id, 2);
    Ok(())
}

/// Check one work for naming and reference problems, recursing into parts.
fn check_work(work: &WorkEntity, context: &mut CheckContext<'_>) -> Result<()> {
    if work
        .name
        .0
        .get("generic")
        .is_none_or(|name| name.trim().is_empty())
    {
        bail!("Work \"{}\" has no generic name", work.key);
    }

    for composer in &work.composers {
        let (person_id, role_id) = match composer {
            ComposerRef::Person(person_id) => (person_id.as_str(), None),
            ComposerRef::Credited { key, role } => (key.as_str(), role.as_deref()),
        };

        if !context.persons.contains(person_id) {
            bail!(
                "Work \"{}\" credits unknown person \"{person_id}\"",
                work.key
            );
        }

        if let Some(role_id) = role_id {
            if !context.roles.contains(role_id) {
                bail!("Work \"{}\" credits unknown role \"{role_id}\"", work.key);
            }
        }
    }

    for instrument in &work.instruments {
        if !context.instruments.contains(instrument) {
            bail!(
                "Work \"{}\" uses unknown instrument \"{instrument}\"",
                work.key
            );
        }
    }

    for tag in &work.tags {
        let (tag_id, value) = match tag {
            TagRef::Key(tag_id) => (tag_id.as_str(), None),
            TagRef::Value { key, value } => (key.as_str(), Some(value)),
        };

        let Some(takes_value) = context.tag_takes_value.get(tag_id) else {
            bail!("Work \"{}\" uses unknown tag \"{tag_id}\"", work.key);
        };

        if *takes_value && value.is_none() {
            context.warnings.push(format!(
                "Work \"{}\": tag \"{tag_id}\" takes a value but none is given",
                work.key
            ));
        } else if !*takes_value && value.is_some() {
            context.warnings.push(format!(
                "Work \"{}\": tag \"{tag_id}\" does not take a value but one is given",
                work.key
            ));
        }
    }

    for part in &work.parts {
        check_work(part, context)?;
    }

    Ok(())
}

/// Write the metadata database, returning what went into it.
fn write_database(metadata: &MetadataFile, works: &WorksFile, output: &Path) -> Result<Summary> {
    let now = db::now();

    if output.exists() {
        fs::remove_file(output)
            .with_context(|| format!("Failed to remove {}", output.display()))?;
    }

    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }

    let path = output
        .to_str()
        .ok_or_else(|| anyhow!("Output path {} is not valid UTF-8", output.display()))?;

    // Runs the migrations and enables foreign key checks, exactly like the
    // Musicus libraries the app opens.
    let mut connection = db::connect(path).context("Failed to create the metadata database")?;

    connection
        .transaction::<(), anyhow::Error, _>(|connection| {
            // The basic entities first: every reference from a work points at
            // one of these.
            for person in &metadata.persons {
                diesel::insert_into(persons::table)
                    .values(tables::Person {
                        person_id: person.key.clone(),
                        name: person.name.clone(),
                        source: Source::Metadata,
                        enable_updates: true,
                        created_at: now,
                        edited_at: now,
                        last_used_at: now,
                    })
                    .execute(connection)?;
            }

            for role in &metadata.roles {
                diesel::insert_into(roles::table)
                    .values(tables::Role {
                        role_id: role.key.clone(),
                        name: role.name.clone(),
                        source: Source::Metadata,
                        enable_updates: true,
                        created_at: now,
                        edited_at: now,
                        last_used_at: now,
                    })
                    .execute(connection)?;
            }

            for instrument in &metadata.instruments {
                diesel::insert_into(instruments::table)
                    .values(tables::Instrument {
                        instrument_id: instrument.key.clone(),
                        name: instrument.name.clone(),
                        source: Source::Metadata,
                        enable_updates: true,
                        created_at: now,
                        edited_at: now,
                        last_used_at: now,
                    })
                    .execute(connection)?;
            }

            for ensemble in &metadata.ensembles {
                diesel::insert_into(ensembles::table)
                    .values(tables::Ensemble {
                        ensemble_id: ensemble.key.clone(),
                        name: ensemble.name.clone(),
                        source: Source::Metadata,
                        enable_updates: true,
                        created_at: now,
                        edited_at: now,
                        last_used_at: now,
                    })
                    .execute(connection)?;
            }

            for tag in &metadata.tags {
                diesel::insert_into(tags::table)
                    .values(tables::Tag {
                        tag_id: tag.key.clone(),
                        name: tag.name.clone(),
                        takes_value: tag.takes_value,
                        source: Source::Metadata,
                        enable_updates: true,
                        created_at: now,
                        edited_at: now,
                        last_used_at: now,
                        private: false,
                    })
                    .execute(connection)?;
            }

            // The works, parents before children, with each work's
            // associations. The related-work links wait until every work
            // exists, because the target may come later in the file.
            for work in &works.works {
                insert_work(connection, work, None, None, now)?;
            }

            for work in &works.works {
                set_relates_to(connection, work)?;
            }

            Ok(())
        })
        .context("Failed to write the metadata database")?;

    Ok(Summary {
        persons: metadata.persons.len(),
        roles: metadata.roles.len(),
        instruments: metadata.instruments.len(),
        ensembles: metadata.ensembles.len(),
        tags: metadata.tags.len(),
        works: count_associations(&works.works, |_| 1),
        composer_credits: count_associations(&works.works, |work| work.composers.len()),
        tag_associations: count_associations(&works.works, |work| work.tags.len()),
        instrumentations: count_associations(&works.works, |work| work.instruments.len()),
        derived: count_associations(&works.works, |work| {
            usize::from(work.related_work.is_some())
        }),
    })
}

/// Insert one work and, recursively, its parts and associations.
///
/// `sequence_number` is the part's position within its parent and is `None`
/// for top-level works.
fn insert_work(
    connection: &mut SqliteConnection,
    work: &WorkEntity,
    parent_work_id: Option<&str>,
    sequence_number: Option<i32>,
    now: NaiveDateTime,
) -> Result<()> {
    diesel::insert_into(works::table)
        .values(tables::Work {
            work_id: work.key.clone(),
            parent_work_id: parent_work_id.map(str::to_owned),
            sequence_number,
            name: work.name.clone(),
            source: Source::Metadata,
            enable_updates: true,
            created_at: now,
            edited_at: now,
            last_used_at: now,
            relates_to: None,
        })
        .execute(connection)?;

    for (index, part) in work.parts.iter().enumerate() {
        insert_work(connection, part, Some(&work.key), Some(index as i32), now)?;
    }

    insert_work_tags(connection, work)?;
    insert_work_persons(connection, work)?;
    insert_work_instruments(connection, work)?;

    Ok(())
}

/// Insert this work's tag associations.
fn insert_work_tags(connection: &mut SqliteConnection, work: &WorkEntity) -> Result<()> {
    for (index, tag) in work.tags.iter().enumerate() {
        let (tag_id, value) = match tag {
            TagRef::Key(tag_id) => (tag_id.as_str(), None),
            TagRef::Value { key, value } => (key.as_str(), Some(value.clone())),
        };

        diesel::insert_into(work_tags::table)
            .values(tables::WorkTag {
                work_id: work.key.clone(),
                tag_id: tag_id.to_owned(),
                value,
                sequence_number: index as i32,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Insert this work's composer credits.
fn insert_work_persons(connection: &mut SqliteConnection, work: &WorkEntity) -> Result<()> {
    for (index, composer) in work.composers.iter().enumerate() {
        let (person_id, role_id) = match composer {
            ComposerRef::Person(person_id) => (person_id.as_str(), None),
            ComposerRef::Credited { key, role } => (key.as_str(), role.clone()),
        };

        diesel::insert_into(work_persons::table)
            .values(tables::WorkPerson {
                work_id: work.key.clone(),
                person_id: person_id.to_owned(),
                role_id,
                sequence_number: index as i32,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Insert this work's instrumentation.
fn insert_work_instruments(connection: &mut SqliteConnection, work: &WorkEntity) -> Result<()> {
    for (index, instrument) in work.instruments.iter().enumerate() {
        diesel::insert_into(work_instruments::table)
            .values(tables::WorkInstrument {
                work_id: work.key.clone(),
                instrument_id: instrument.clone(),
                sequence_number: index as i32,
            })
            .execute(connection)?;
    }

    Ok(())
}

/// Point each derivative at its related work, recursing into parts.
fn set_relates_to(connection: &mut SqliteConnection, work: &WorkEntity) -> Result<()> {
    if let Some(related) = &work.related_work {
        diesel::update(works::table.filter(works::work_id.eq(&work.key)))
            .set(works::relates_to.eq(related.as_str()))
            .execute(connection)?;
    }

    for part in &work.parts {
        set_relates_to(connection, part)?;
    }

    Ok(())
}

/// Sum a per-work measure over the whole hierarchy.
fn count_associations(
    works: &[WorkEntity],
    mut measure: impl FnMut(&WorkEntity) -> usize,
) -> usize {
    fn count(works: &[WorkEntity], measure: &mut impl FnMut(&WorkEntity) -> usize) -> usize {
        works.iter().fold(0, |total, work| {
            total + measure(work) + count(&work.parts, measure)
        })
    }

    count(works, &mut measure)
}

#[cfg(test)]
mod tests {
    use musicus_library::db::schema::*;

    use super::*;

    const METADATA: &str = r#"{
        "roles": [
            { "key": "arranger", "name": { "generic": "Arranger" } }
        ],
        "instruments": [
            { "key": "violin", "name": { "generic": "Violin", "de": "Violine" } }
        ],
        "ensembles": [
            { "key": "some-ensemble", "name": { "generic": "Some Ensemble" } }
        ],
        "persons": [
            { "key": "bach-johann-sebastian", "name": { "generic": "Johann Sebastian Bach" } }
        ],
        "tags": [
            { "key": "baroque", "name": { "generic": "Baroque" } },
            { "key": "language", "name": { "generic": "Language" }, "takes_value": true }
        ]
    }"#;

    const WORKS: &str = r#"{
        "works": [
            {
                "key": "suite",
                "name": { "generic": "A Suite", "de": "Eine Suite" },
                "composers": ["bach-johann-sebastian"],
                "tags": ["baroque", { "key": "language", "value": { "generic": "Latin" } }],
                "instruments": ["violin"],
                "parts": [
                    {
                        "key": "suite-m1",
                        "name": { "generic": "1. Prelude" },
                        "instruments": ["violin"]
                    }
                ]
            },
            {
                "key": "suite-arrangement",
                "name": { "generic": "An Arrangement" },
                "composers": [
                    { "key": "bach-johann-sebastian", "role": "arranger" }
                ],
                "relatedWork": "suite"
            }
        ]
    }"#;

    fn metadata() -> MetadataFile {
        serde_json::from_str(METADATA).unwrap()
    }

    fn works() -> WorksFile {
        serde_json::from_str(WORKS).unwrap()
    }

    #[test]
    fn accepted_definitions_validate() {
        let validation = validate(&metadata(), &works()).unwrap();
        assert!(validation.warnings.is_empty());
    }

    #[test]
    fn missing_generic_name_is_rejected() {
        let mut metadata = metadata();
        metadata.persons[0].name = TranslatedString::default();

        let err = validate(&metadata, &works()).unwrap_err();
        assert!(err.to_string().contains("no generic name"));
    }

    #[test]
    fn duplicate_entity_key_is_rejected() {
        let mut metadata = metadata();
        metadata.persons.push(metadata.persons[0].clone());

        let err = validate(&metadata, &works()).unwrap_err();
        assert!(err.to_string().contains("Duplicate person key"));
    }

    #[test]
    fn unknown_reference_is_rejected() {
        let mut works = works();
        works.works[0].instruments.push("theremin".to_string());

        let err = validate(&metadata(), &works).unwrap_err();
        assert!(err.to_string().contains("unknown instrument"));
    }

    #[test]
    fn unrelated_work_is_rejected() {
        let mut works = works();
        works.works[0].related_work = Some("missing-work".to_string());

        let err = validate(&metadata(), &works).unwrap_err();
        assert!(err.to_string().contains("unknown work"));
    }

    #[test]
    fn related_work_cycle_is_rejected() {
        let mut works = works();
        // The top-level work now points back at the arrangement and the
        // arrangement at the suite: a cycle through "suite".
        works.works[0].related_work = Some("suite-arrangement".to_string());

        let err = validate(&metadata(), &works).unwrap_err();
        assert!(err.to_string().contains("form a cycle"));
    }

    #[test]
    fn tag_value_mismatch_is_a_warning() {
        let mut works = works();
        works.works[0].tags[0] = TagRef::Value {
            key: "baroque".to_string(),
            value: TranslatedString::generic("too much"),
        };

        let validation = validate(&metadata(), &works).unwrap();
        assert_eq!(validation.warnings.len(), 1);
        assert!(validation
            .warnings
            .first()
            .unwrap()
            .contains("does not take a value"));
    }

    #[test]
    fn the_database_round_trips_through_musicus_library() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("metadata.musdb");

        let output = build(&metadata(), &works(), &path).unwrap();
        let summary = output.summary;

        assert_eq!(summary.persons, 1);
        assert_eq!(summary.roles, 1);
        assert_eq!(summary.instruments, 1);
        assert_eq!(summary.ensembles, 1);
        assert_eq!(summary.tags, 2);
        assert_eq!(summary.works, 3);
        assert_eq!(summary.composer_credits, 2);
        assert_eq!(summary.tag_associations, 2);
        assert_eq!(summary.instrumentations, 2);
        assert_eq!(summary.derived, 1);

        let path = path.to_str().unwrap();
        let mut connection = db::connect(path).unwrap();

        let persons = persons::table
            .load::<tables::Person>(&mut connection)
            .unwrap();
        assert_eq!(persons.len(), 1);
        assert_eq!(persons[0].person_id, "bach-johann-sebastian");
        assert_eq!(persons[0].source, Source::Metadata);
        assert_eq!(persons[0].name.get(), "Johann Sebastian Bach");

        // The hierarchy: the movement hangs under the suite at position 0.
        let movement = works::table
            .filter(works::work_id.eq("suite-m1"))
            .first::<tables::Work>(&mut connection)
            .unwrap();
        assert_eq!(movement.parent_work_id.as_deref(), Some("suite"));
        assert_eq!(movement.sequence_number, Some(0));

        // The value tag keeps its translated value.
        let language = work_tags::table
            .filter(work_tags::work_id.eq("suite"))
            .filter(work_tags::tag_id.eq("language"))
            .first::<tables::WorkTag>(&mut connection)
            .unwrap();
        assert_eq!(language.value.unwrap().get(), "Latin");

        // The credited composer carries the role.
        let credited = work_persons::table
            .filter(work_persons::work_id.eq("suite-arrangement"))
            .first::<tables::WorkPerson>(&mut connection)
            .unwrap();
        assert_eq!(credited.role_id.as_deref(), Some("arranger"));

        // The related work link points back at the main version.
        let arrangement = works::table
            .filter(works::work_id.eq("suite-arrangement"))
            .first::<tables::Work>(&mut connection)
            .unwrap();
        assert_eq!(arrangement.relates_to.as_deref(), Some("suite"));
    }

    #[test]
    fn an_existing_output_is_replaced() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("metadata.musdb");
        fs::write(&path, "not a database").unwrap();

        let output = build(&metadata(), &works(), &path).unwrap();
        assert_eq!(output.summary.works, 3);

        let database = fs::read(&path).unwrap();
        assert!(!database.starts_with(b"not a database"));
    }
}
