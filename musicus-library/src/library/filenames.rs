//! Human interpretable file names for track files.
//!
//! Track files are copied into the library folder under a name derived from a
//! user configurable pattern, so that the folder stays readable outside of
//! Musicus. Nothing depends on those names: the database keeps the authoritative
//! path of every track, and a name that cannot be built falls back to the
//! track's identifiers.

use anyhow::{anyhow, Result};
use formatx::Template;

use crate::db::models::{Recording, Work};

/// The pattern used unless the user configured another one.
///
/// This must stay in sync with the default of the `track-filename-pattern`
/// setting in `data/de.johrpan.Musicus.gschema.xml.in`.
pub const DEFAULT_FILENAME_PATTERN: &str = "{composer}; {work}; {index} {part}";

/// The placeholders a pattern may use.
pub const PLACEHOLDERS: &[&str] = &[
    "composer",
    "work",
    "part",
    "performers",
    "index",
    "year",
];

/// The longest file stem that will be generated.
///
/// Every relevant file system allows at least 255 bytes per file name. The
/// remaining room is for the deduplication suffix, the file extension and the
/// `.part` suffix a track file carries while it is being imported.
const MAX_STEM_LENGTH: usize = 180;

/// Characters that are not allowed in a file name on Windows, and that would
/// either separate path components or confuse shell users elsewhere.
const ILLEGAL_CHARACTERS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

/// Characters that separate the parts of a name and that may therefore neither
/// begin nor end it.
const SEPARATORS: &[char] = &[' ', '_', '-', '.'];

/// File stems that Windows reserves for devices, regardless of extension.
const RESERVED_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The values a filename pattern can refer to.
pub struct TrackNameData {
    pub composer: String,
    pub work: String,
    pub part: String,
    pub performers: String,
    pub index: String,
    pub year: String,
}

impl TrackNameData {
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
            year: recording
                .year()
                .map(|year| year.to_string())
                .unwrap_or_default(),
        }
    }
}

/// Build the file stem for `data` from `pattern`.
///
/// Returns `None` if the pattern is unusable or describes a name that is empty
/// once it has been made safe to use as a file name. Callers are expected to
/// fall back to a name that is always available in that case, so that no import
/// can fail because of naming.
pub fn render(pattern: &str, data: &TrackNameData) -> Option<String> {
    let stem = sanitize(&render_raw(pattern, data).ok()?);

    if stem.is_empty() {
        None
    } else {
        Some(stem)
    }
}

/// The file stem for a track whose name cannot be built from the pattern.
///
/// It is derived from identifiers only, so it is always available and unique
/// per track position.
pub fn fallback_stem(recording_id: &str, recording_index: i32) -> String {
    format!("{recording_id}_{recording_index:02}")
}

/// Whether `pattern` can be used as a filename pattern.
///
/// The error describes what is wrong with the pattern and is meant to be shown
/// to the user.
pub fn validate(pattern: &str) -> Result<()> {
    preview(pattern).map(|_| ())
}

/// The name `pattern` would produce for an example track.
///
/// This doubles as validation: a pattern that cannot be rendered for the
/// example cannot be rendered for a real track either.
pub fn preview(pattern: &str) -> Result<String> {
    let data = TrackNameData {
        composer: "Ludwig van Beethoven".to_owned(),
        work: "Symphony No. 5 in C minor, Op. 67".to_owned(),
        part: "Allegro con brio".to_owned(),
        performers: "Wiener Philharmoniker".to_owned(),
        index: "01".to_owned(),
        year: "1977".to_owned(),
    };

    let stem = sanitize(&render_raw(pattern, &data)?);

    if stem.is_empty() {
        return Err(anyhow!("The pattern does not describe a file name."));
    }

    Ok(format!("{stem}.flac"))
}

/// Substitute the placeholders of `pattern` without making the result safe to
/// use as a file name.
fn render_raw(pattern: &str, data: &TrackNameData) -> Result<String> {
    let template = Template::new(pattern).map_err(|err| anyhow!("{err}"))?;

    check_placeholders(pattern)?;

    template
        .render()
        .named("composer", &data.composer)
        .named("work", &data.work)
        .named("part", &data.part)
        .named("performers", &data.performers)
        .named("index", &data.index)
        .named("year", &data.year)
        .finish()
        .map_err(|err| anyhow!("{err}"))
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

/// Reduce `name` to a portable ASCII file stem.
fn sanitize(name: &str) -> String {
    // Transliterating first is what makes the result portable, but it can
    // introduce illegal characters of its own ("½" becomes "1/2"), so the
    // replacement below has to happen afterwards.
    let transliterated = deunicode::deunicode(name);

    let replaced = transliterated
        .chars()
        .map(|character| {
            if ILLEGAL_CHARACTERS.contains(&character)
                || character.is_control()
                || !character.is_ascii()
            {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();

    // A placeholder without a value leaves the separators around it behind.
    // Collapsing repetitions of one and the same separator removes those
    // without touching intentional separators like the " - " between two
    // movement names.
    let mut collapsed = String::with_capacity(replaced.len());
    let mut previous: Option<char> = None;

    for character in replaced.chars() {
        if SEPARATORS.contains(&character) && previous == Some(character) {
            continue;
        }

        collapsed.push(character);
        previous = Some(character);
    }

    let mut stem = collapsed
        .trim_matches(|c| SEPARATORS.contains(&c))
        .to_owned();

    if stem.len() > MAX_STEM_LENGTH {
        let end = (0..=MAX_STEM_LENGTH)
            .rev()
            .find(|index| stem.is_char_boundary(*index))
            .unwrap_or(0);

        stem.truncate(end);
        stem = stem.trim_matches(|c| SEPARATORS.contains(&c)).to_owned();
    }

    if RESERVED_STEMS.contains(&stem.to_uppercase().as_str()) {
        stem.insert(0, '_');
    }

    stem
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::db::TranslatedString;

    fn data() -> TrackNameData {
        TrackNameData {
            composer: "Antonín Dvořák".to_owned(),
            work: "Symfonie č. 9".to_owned(),
            part: "Largo".to_owned(),
            performers: "Berliner Philharmoniker".to_owned(),
            index: "02".to_owned(),
            year: "1993".to_owned(),
        }
    }

    fn translated(name: &str) -> TranslatedString {
        let mut translations = HashMap::new();
        translations.insert("generic".to_string(), name.to_string());
        TranslatedString(translations)
    }

    #[test]
    fn default_pattern_renders_a_readable_name() {
        assert_eq!(
            render(DEFAULT_FILENAME_PATTERN, &data()).unwrap(),
            "Antonin Dvorak_Symfonie c. 9_02 Largo"
        );
    }

    #[test]
    fn all_placeholders_are_supported() {
        let pattern = PLACEHOLDERS
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<Vec<String>>()
            .join(" ");

        assert!(validate(&pattern).is_ok());
        assert!(render(&pattern, &data()).unwrap().contains("1993"));
    }

    #[test]
    fn illegal_characters_are_replaced() {
        let mut data = data();
        data.work = "Prelude: C/D \"draft\" <1>?*|\\".to_owned();

        let stem = render("{work}", &data).unwrap();

        assert!(!stem.contains(ILLEGAL_CHARACTERS));
        assert_eq!(stem, "Prelude C D draft 1");
    }

    #[test]
    fn transliteration_can_introduce_illegal_characters() {
        let mut data = data();
        data.work = "½ Sonate".to_owned();

        assert_eq!(render("{work}", &data).unwrap(), "1 2 Sonate");
    }

    #[test]
    fn control_characters_are_replaced() {
        let mut data = data();
        data.work = "Line\nbreak\ttab".to_owned();

        assert_eq!(render("{work}", &data).unwrap(), "Line break tab");
    }

    #[test]
    fn empty_values_do_not_leave_separators_behind() {
        let mut data = data();
        data.composer = String::new();

        assert_eq!(
            render(DEFAULT_FILENAME_PATTERN, &data).unwrap(),
            "Symfonie c. 9_02 Largo"
        );

        let mut data = data;
        data.work = String::new();

        assert_eq!(render(DEFAULT_FILENAME_PATTERN, &data).unwrap(), "02 Largo");
    }

    #[test]
    fn intentional_separators_are_kept() {
        let mut data = data();
        data.part = "Adagio - Allegro".to_owned();

        assert_eq!(render("{movement}", &data).unwrap(), "Adagio - Allegro");
    }

    #[test]
    fn long_names_are_truncated_on_a_character_boundary() {
        let mut data = data();
        data.work = "Sinfonía ".repeat(50);

        let stem = render("{work}", &data).unwrap();

        assert!(stem.len() <= MAX_STEM_LENGTH);
        assert!(stem.starts_with("Sinfonia Sinfonia"));
        assert!(!stem.ends_with(' '));
    }

    #[test]
    fn reserved_stems_are_escaped() {
        let mut data = data();
        data.work = "nul".to_owned();

        assert_eq!(render("{work}", &data).unwrap(), "_nul");
    }

    #[test]
    fn a_name_that_sanitizes_to_nothing_is_rejected() {
        let mut data = data();
        data.work = "///".to_owned();

        assert!(render("{work}", &data).is_none());
        assert!(validate("///").is_err());
    }

    #[test]
    fn unknown_placeholders_are_rejected() {
        let error = validate("{composer} {bogus}").unwrap_err().to_string();

        assert!(error.contains("bogus"), "{error}");
        assert!(render("{bogus}", &data()).is_none());
    }

    #[test]
    fn malformed_patterns_are_rejected() {
        assert!(validate("{composer").is_err());
        assert!(validate("{}").is_err());
    }

    #[test]
    fn the_default_pattern_is_valid() {
        assert_eq!(
            preview(DEFAULT_FILENAME_PATTERN).unwrap(),
            "Ludwig van Beethoven_Symphony No. 5 in C minor, Op. 67_01 Allegro con brio.flac"
        );
    }

    #[test]
    fn track_name_data_is_taken_from_the_recording() {
        let work = Work {
            work_id: "work".to_owned(),
            name: translated("Symphony No. 5"),
            parts: Vec::new(),
            persons: Vec::new(),
            instruments: Vec::new(),
            tags: Vec::new(),
            enable_updates: false,
        };

        let part = Work {
            work_id: "part".to_owned(),
            name: translated("Allegro"),
            ..work.clone()
        };

        let recording = Recording {
            recording_id: "recording".to_owned(),
            work,
            persons: Vec::new(),
            ensembles: Vec::new(),
            tags: Vec::new(),
            enable_updates: false,
        };

        let data = TrackNameData::new(&recording, 0, std::slice::from_ref(&part));

        assert_eq!(data.work, "Symphony No. 5");
        assert_eq!(data.part, "Allegro");
        assert_eq!(data.index, "01");
        assert!(data.composer.is_empty());
        assert!(data.year.is_empty());
    }
}
