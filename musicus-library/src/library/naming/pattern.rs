//! The placeholder patterns that describe how a track file is named and tagged.
//!
//! The same handful of placeholders drives the file name and every tag, so that
//! the user only has to learn one vocabulary and a pattern can be moved from one
//! setting to another. Everything specific to one target — making a name safe
//! for a file system, deciding what an empty tag means — lives in [`filenames`]
//! and [`audio_tags`] respectively.
//!
//! [`filenames`]: super::filenames
//! [`audio_tags`]: super::audio_tags

use anyhow::{anyhow, Result};
use formatx::Template;

use crate::db::models::{Recording, Work};

/// The file name pattern used unless the user configured another one.
///
/// This must stay in sync with the default of the `track-filename-pattern`
/// setting in `data/de.johrpan.Musicus.gschema.xml.in`.
pub const DEFAULT_FILENAME_PATTERN: &str = "{composer}; {work}; {index} {part}";

/// The album tag pattern used unless the user configured another one.
///
/// This must stay in sync with the default of the `track-tag-album-pattern`
/// setting in `data/de.johrpan.Musicus.gschema.xml.in`.
pub const DEFAULT_ALBUM_PATTERN: &str = "{composer}";

/// The artist tag pattern used unless the user configured another one.
///
/// This must stay in sync with the default of the `track-tag-artist-pattern`
/// setting in `data/de.johrpan.Musicus.gschema.xml.in`.
pub const DEFAULT_ARTIST_PATTERN: &str = "{performers}";

/// The title tag pattern used unless the user configured another one.
///
/// The track number is written as a tag of its own, so repeating the index here
/// would show up twice in players that display both.
///
/// This must stay in sync with the default of the `track-tag-title-pattern`
/// setting in `data/de.johrpan.Musicus.gschema.xml.in`.
pub const DEFAULT_TITLE_PATTERN: &str = "{work}: {part}";

/// The placeholders a pattern may use.
pub const PLACEHOLDERS: &[&str] = &["composer", "work", "part", "performers", "index"];

/// Every pattern that describes a track file.
///
/// They are configured together by the user and pushed into the library as a
/// whole, so that they cannot drift apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Patterns {
    pub filename: String,
    pub album: String,
    pub artist: String,
    pub title: String,
}

impl Default for Patterns {
    fn default() -> Self {
        Self {
            filename: DEFAULT_FILENAME_PATTERN.to_owned(),
            album: DEFAULT_ALBUM_PATTERN.to_owned(),
            artist: DEFAULT_ARTIST_PATTERN.to_owned(),
            title: DEFAULT_TITLE_PATTERN.to_owned(),
        }
    }
}

/// The values a pattern can refer to.
pub struct TrackData {
    pub composer: String,
    pub work: String,
    pub part: String,
    pub performers: String,
    pub index: String,
}

impl TrackData {
    /// Collect the values describing the track at `recording_index` of
    /// `recording` that covers `works`.
    pub fn new(recording: &Recording, recording_index: i32, works: &[Work]) -> Self {
        Self {
            composer: recording.work.composers_string().unwrap_or_default(),
            work: recording.work.name.get().to_owned(),
            part: works
                .iter()
                .map(|work| work.name.get().to_owned())
                .collect::<Vec<String>>()
                .join(", "),
            performers: recording.performers_string(),
            index: format!("{:02}", recording_index + 1),
        }
    }

    /// The track that previews are rendered for.
    ///
    /// Every preview shown to the user describes the same track, so that the
    /// file name and the tags can be compared against each other.
    pub fn example() -> Self {
        Self {
            composer: "Ludwig van Beethoven".to_owned(),
            work: "Symphony No. 5 in C minor, Op. 67".to_owned(),
            part: "Allegro con brio".to_owned(),
            performers: "Wiener Philharmoniker".to_owned(),
            index: "01".to_owned(),
        }
    }
}

/// Substitute the placeholders of `pattern` with the values of `data`.
///
/// The result is raw text. Making it usable as a file name or as a tag value is
/// the caller's job.
pub fn render(pattern: &str, data: &TrackData) -> Result<String> {
    let template = Template::new(pattern).map_err(|err| anyhow!("{err}"))?;

    check_placeholders(pattern)?;

    template
        .render()
        .named("composer", &data.composer)
        .named("work", &data.work)
        .named("part", &data.part)
        .named("performers", &data.performers)
        .named("index", &data.index)
        .finish()
        .map_err(|err| anyhow!("{err}"))
}

/// Whether `pattern` can be rendered at all.
///
/// The error describes what is wrong with the pattern and is meant to be shown
/// to the user. Whether the rendered text is usable for its purpose is decided
/// by the caller.
pub fn validate(pattern: &str) -> Result<()> {
    render(pattern, &TrackData::example()).map(|_| ())
}

/// Drop repetitions of one and the same separator character.
///
/// A placeholder without a value leaves the separators around it behind. This
/// removes those without touching intentional separators like the " - " between
/// two movement names, which are made up of different characters.
pub(super) fn collapse_separators(text: &str, separators: &[char]) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut previous: Option<char> = None;

    for character in text.chars() {
        if separators.contains(&character) && previous == Some(character) {
            continue;
        }

        collapsed.push(character);
        previous = Some(character);
    }

    collapsed
}

/// Ensure that every placeholder of `pattern` refers to one of the values a
/// track provides.
///
/// The template itself only rejects placeholders it cannot parse, and renders a
/// positional one as an empty string, which would silently swallow part of the
/// user's pattern.
fn check_placeholders(pattern: &str) -> Result<()> {
    let mut rest = pattern;

    while let Some(start) = rest.find('{') {
        // An escaped brace is literal text and does not open a placeholder.
        if rest[start..].starts_with("{{") {
            rest = &rest[start + 2..];
            continue;
        }

        let body = &rest[start + 1..];
        let end = body
            .find('}')
            .ok_or_else(|| anyhow!("A placeholder is missing its closing brace."))?;

        let name = body[..end].split(':').next().unwrap_or_default();

        if !PLACEHOLDERS.contains(&name) {
            return Err(anyhow!("Unknown placeholder: {{{name}}}"));
        }

        rest = &body[end + 1..];
    }

    Ok(())
}
