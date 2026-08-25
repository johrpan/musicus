//! Human interpretable file names for track files.
//!
//! Track files are copied into the library folder under a name derived from a
//! user configurable pattern, so that the folder stays readable outside of
//! Musicus. Nothing depends on those names: the database keeps the authoritative
//! path of every track, and a name that cannot be built falls back to the
//! track's identifiers.
//!
//! The pattern vocabulary itself lives in [`pattern`](super::pattern); this
//! module only turns rendered text into something a file system accepts.

use anyhow::{anyhow, Result};

use super::pattern;

pub use super::pattern::{TrackData as TrackNameData, DEFAULT_FILENAME_PATTERN, PLACEHOLDERS};

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
const SEPARATORS: &[char] = &[' ', '_', '-', '.', ';', ','];

/// File stems that Windows reserves for devices, regardless of extension.
const RESERVED_STEMS: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Build the file stem for `data` from `pattern`.
///
/// Returns `None` if the pattern is unusable or describes a name that is empty
/// once it has been made safe to use as a file name. Callers are expected to
/// fall back to a name that is always available in that case, so that no import
/// can fail because of naming.
pub fn render(pattern: &str, data: &TrackNameData) -> Option<String> {
    let stem = sanitize(&pattern::render(pattern, data).ok()?);

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
    let stem = sanitize(&pattern::render(pattern, &TrackNameData::example())?);

    if stem.is_empty() {
        return Err(anyhow!("The pattern does not describe a file name."));
    }

    Ok(format!("{stem}.flac"))
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

    let mut stem = pattern::collapse_separators(&replaced, SEPARATORS)
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
