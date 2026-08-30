//! The JSON format that describes a Musicus metadata catalogue and, within
//! the same format, the recordings a library imports from it.
//!
//! There is no dedicated "metadata file" or "works file": every source file
//! is a [`Definitions`], and which top-level keys it uses (`persons`,
//! `works`, `recordings`, ...) is entirely up to how the content is split up.
//! Any number of files can be parsed and folded together with
//! [`Definitions::merge`] before use. [`build`]/[`validate`] only look at the
//! catalogue fields (`persons` through `works`) and ignore `recordings`;
//! `recordings` is for `musicus_library::catalog_import` instead.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::Deserialize;

use crate::{
    db::{self, schema::*, tables, tables::Source, TranslatedString},
    error::Result,
};

/// One named basic entity (`roles`, `instruments`, `ensembles`, `persons`).
#[derive(Clone, Debug, Deserialize)]
pub struct NamedEntity {
    pub key: String,
    pub name: TranslatedString,
}

/// A tag definition. Tags additionally carry whether associations with them
/// take a user-defined value.
#[derive(Clone, Debug, Deserialize)]
pub struct TagEntity {
    pub key: String,
    pub name: TranslatedString,
    #[serde(default)]
    pub takes_value: bool,
}

/// A composer credit on a work: a plain person key, or a person key with a
/// role for contributions that are not plain composition (such as the
/// `compiler` of the *Hexameron*).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ComposerRef {
    Person(String),
    Credited { key: String, role: Option<String> },
}

/// A tag association on a work: a plain tag key, or a tag key with a value
/// for tags that take one (such as `Language`).
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum TagRef {
    Key(String),
    Value {
        key: String,
        value: TranslatedString,
    },
}

/// A work entry; `parts` nests recursively.
#[derive(Clone, Debug, Deserialize)]
pub struct WorkEntity {
    pub key: String,
    pub name: TranslatedString,
    #[serde(default)]
    pub composers: Vec<ComposerRef>,
    #[serde(default)]
    pub tags: Vec<TagRef>,
    #[serde(default)]
    pub instruments: Vec<String>,
    #[serde(default)]
    pub parts: Vec<WorkEntity>,
    /// The key of the work this one is derived from.
    #[serde(rename = "relatedWork", default)]
    pub related_work: Option<String>,
}

/// A person or ensemble performing on a recording, referenced by catalogue
/// key.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum PerformerRef {
    Person {
        person: String,
        role: Option<String>,
        instrument: Option<String>,
    },
    Ensemble {
        ensemble: String,
        role: Option<String>,
    },
}

/// One track: an audio file on disk, plus the works (by catalogue key) it
/// belongs to, in order.
#[derive(Clone, Debug, Deserialize)]
pub struct TrackEntry {
    pub file: PathBuf,
    #[serde(default)]
    pub works: Vec<String>,
}

/// One recording: a catalogue work, its performers and tags, and its tracks.
#[derive(Clone, Debug, Deserialize)]
pub struct RecordingEntry {
    pub work: String,
    #[serde(default)]
    pub performers: Vec<PerformerRef>,
    #[serde(default)]
    pub tags: Vec<TagRef>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub tracks: Vec<TrackEntry>,
}

/// The entity definitions of one or more source files, folded together.
///
/// Every field is independently optional, so a single file may define
/// anything from just a handful of tags to a whole catalogue plus its
/// recordings; how the content is split across files is not meaningful.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Definitions {
    #[serde(default)]
    pub persons: Vec<NamedEntity>,
    #[serde(default)]
    pub roles: Vec<NamedEntity>,
    #[serde(default)]
    pub instruments: Vec<NamedEntity>,
    #[serde(default)]
    pub ensembles: Vec<NamedEntity>,
    #[serde(default)]
    pub tags: Vec<TagEntity>,
    #[serde(default)]
    pub works: Vec<WorkEntity>,
    #[serde(default)]
    pub recordings: Vec<RecordingEntry>,
}

impl Definitions {
    /// Fold several sources into one, in order, keeping every entry: a
    /// duplicate key is a validation error, not something merging should
    /// silently resolve.
    pub fn merge(sources: impl IntoIterator<Item = Definitions>) -> Definitions {
        let mut merged = Definitions::default();

        for source in sources {
            merged.persons.extend(source.persons);
            merged.roles.extend(source.roles);
            merged.instruments.extend(source.instruments);
            merged.ensembles.extend(source.ensembles);
            merged.tags.extend(source.tags);
            merged.works.extend(source.works);
            merged.recordings.extend(source.recordings);
        }

        merged
    }
}

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

/// Check the catalogue fields of `definitions` against each other, ignoring
/// `recordings`.
///
/// Format violations are errors. Problems the database cannot represent
/// faithfully (for example a value on a tag that does not take one) are
/// warnings.
pub fn validate(definitions: &Definitions) -> Result<Validation> {
    Ok(validate_inner(definitions)?)
}

fn validate_inner(definitions: &Definitions) -> anyhow::Result<Validation> {
    let persons = unique_keys(&definitions.persons, "person", |entry| &entry.key)?;
    let roles = unique_keys(&definitions.roles, "role", |entry| &entry.key)?;
    let instruments = unique_keys(&definitions.instruments, "instrument", |entry| &entry.key)?;
    let ensembles = unique_keys(&definitions.ensembles, "ensemble", |entry| &entry.key)?;
    unique_keys(&definitions.tags, "tag", |entry| &entry.key)?;

    let tag_takes_value = definitions
        .tags
        .iter()
        .map(|tag| (tag.key.clone(), tag.takes_value))
        .collect();

    // Works form a single namespace of keys across every nesting level.
    let mut work_keys = HashSet::new();
    index_works(&definitions.works, &mut work_keys)?;

    // Related works must exist; the relation is meant to point from a
    // derivative towards the main version, so a cycle is a mistake. The
    // parent chain cannot cycle (the file is a tree), but the related-work
    // graph is not.
    let mut related_to: HashMap<&str, &str> = HashMap::new();
    collect_related(&definitions.works, &mut related_to);

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
    for work in &definitions.works {
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

/// Build the metadata database at `output` from the catalogue fields of
/// `definitions`, ignoring `recordings`.
///
/// The output file is replaced if it exists; the database is fully derived
/// from `definitions`.
pub fn build(definitions: &Definitions, output: &Path) -> Result<BuildOutput> {
    let validation = validate(definitions)?;
    let summary = write_database(definitions, output)?;

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
) -> anyhow::Result<HashSet<String>> {
    let mut seen = HashSet::new();
    for item in items {
        if !seen.insert(key(item).to_owned()) {
            bail!("Duplicate {kind} key \"{}\"", key(item));
        }
    }
    Ok(seen)
}

/// Index every work at every nesting level by its key.
fn index_works(list: &[WorkEntity], keys: &mut HashSet<String>) -> anyhow::Result<()> {
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
) -> anyhow::Result<()> {
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

/// Check one work for reference problems, recursing into parts.
fn check_work(work: &WorkEntity, context: &mut CheckContext<'_>) -> anyhow::Result<()> {
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
fn write_database(definitions: &Definitions, output: &Path) -> Result<Summary> {
    let now = chrono::Utc::now().naive_utc();

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
            for person in &definitions.persons {
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

            for role in &definitions.roles {
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

            for instrument in &definitions.instruments {
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

            for ensemble in &definitions.ensembles {
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

            for tag in &definitions.tags {
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
            for work in &definitions.works {
                insert_work(connection, work, None, None, now)?;
            }

            for work in &definitions.works {
                insert_relations(connection, work)?;
            }

            Ok(())
        })
        .context("Failed to write the metadata database")?;

    Ok(Summary {
        persons: definitions.persons.len(),
        roles: definitions.roles.len(),
        instruments: definitions.instruments.len(),
        ensembles: definitions.ensembles.len(),
        tags: definitions.tags.len(),
        works: count_associations(&definitions.works, |_| 1),
        composer_credits: count_associations(&definitions.works, |work| work.composers.len()),
        tag_associations: count_associations(&definitions.works, |work| work.tags.len()),
        instrumentations: count_associations(&definitions.works, |work| work.instruments.len()),
        derived: count_associations(&definitions.works, |work| {
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
) -> anyhow::Result<()> {
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
fn insert_work_tags(connection: &mut SqliteConnection, work: &WorkEntity) -> anyhow::Result<()> {
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
fn insert_work_persons(connection: &mut SqliteConnection, work: &WorkEntity) -> anyhow::Result<()> {
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
fn insert_work_instruments(
    connection: &mut SqliteConnection,
    work: &WorkEntity,
) -> anyhow::Result<()> {
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

/// Insert each work's related-work relations, recursing into parts.
///
/// Runs after every work exists, so a relation can point at a work defined
/// later in the file.
fn insert_relations(connection: &mut SqliteConnection, work: &WorkEntity) -> anyhow::Result<()> {
    if let Some(related) = &work.related_work {
        diesel::insert_into(work_relations::table)
            .values(tables::WorkRelation {
                work_id: work.key.clone(),
                related_work_id: related.clone(),
            })
            .execute(connection)?;
    }

    for part in &work.parts {
        insert_relations(connection, part)?;
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
