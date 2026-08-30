//! The JSON data format of the pre-made Musicus metadata database.
//!
//! `metadata.json` holds the basic entities (`roles`, `instruments`,
//! `ensembles`, `persons`, `tags`), `works.json` the work hierarchy. The
//! format is documented in the `format.md` of the `musicus-data` repository.

use std::path::Path;

use anyhow::{Context, Result};
use musicus_library::TranslatedString;
use serde::Deserialize;

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

/// The contents of `metadata.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct MetadataFile {
    #[serde(default)]
    pub roles: Vec<NamedEntity>,
    #[serde(default)]
    pub instruments: Vec<NamedEntity>,
    #[serde(default)]
    pub ensembles: Vec<NamedEntity>,
    #[serde(default)]
    pub persons: Vec<NamedEntity>,
    #[serde(default)]
    pub tags: Vec<TagEntity>,
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

/// The contents of `works.json`.
#[derive(Clone, Debug, Deserialize)]
pub struct WorksFile {
    #[serde(default)]
    pub works: Vec<WorkEntity>,
}

fn read_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let file =
        std::fs::File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    serde_json::from_reader(file).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Read and parse `metadata.json`.
pub fn read_metadata(path: &Path) -> Result<MetadataFile> {
    read_file(path)
}

/// Read and parse `works.json`.
pub fn read_works(path: &Path) -> Result<WorksFile> {
    read_file(path)
}
