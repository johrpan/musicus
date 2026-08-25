//! The tags written into the audio files themselves.
//!
//! Musicus keeps the authoritative metadata in its database; the tags exist so
//! that the files stay meaningful in other players. They are therefore derived
//! from the database and never read back into it.
//!
//! Unlike naming a file, tagging it rewrites the user's file and cannot be
//! undone. A tag that cannot be written is therefore always reported and never
//! fatal: the database is unaffected, and the files can be brought back in line
//! at any time with a reorganization.

use std::path::Path;

use anyhow::{anyhow, bail, Context, Result};
use lofty::{
    config::{ParseOptions, WriteOptions},
    file::TaggedFileExt,
    probe::Probe,
    tag::{Accessor, Tag, TagExt, TagType},
};

use super::pattern::{self, Patterns, TrackData};

pub use super::pattern::{
    DEFAULT_ALBUM_PATTERN, DEFAULT_ARTIST_PATTERN, DEFAULT_TITLE_PATTERN, PLACEHOLDERS,
};

/// Characters that separate the parts of a tag value and that may therefore
/// neither begin nor end it.
///
/// This is deliberately wider than the set used for file names: a tag pattern
/// like `{composer}: {work}` has to survive a recording without a composer.
const SEPARATORS: &[char] = &[' ', '_', '-', '.', ';', ',', ':'];

/// The tags of one track, ready to be written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioTags {
    /// `None` if the pattern produced no value. The tag is then left out
    /// entirely rather than written as an empty string.
    pub album: Option<String>,
    pub artist: Option<String>,
    pub title: Option<String>,
    /// Derived from the position within the recording rather than from a
    /// pattern, because players sort by it numerically.
    pub track_number: u32,
}

impl AudioTags {
    /// Build the tags for the track at `recording_index` described by `data`.
    pub fn render(patterns: &Patterns, data: &TrackData, recording_index: i32) -> Self {
        let render = |pattern: &str| {
            pattern::render(pattern, data)
                .ok()
                .map(|value| tidy(&value))
                .filter(|value| !value.is_empty())
        };

        Self {
            album: render(&patterns.album),
            artist: render(&patterns.artist),
            // A recording whose tracks have no parts of their own renders an
            // empty title under the default pattern. A file without a title is
            // unusable in a player, so it falls back to the work, in the same
            // spirit as `filenames::fallback_stem`. A pattern the user emptied
            // out asks for no title at all and is left alone.
            title: render(&patterns.title).or_else(|| {
                if is_disabled(&patterns.title) {
                    return None;
                }

                let work = tidy(&data.work);
                (!work.is_empty()).then_some(work)
            }),
            track_number: recording_index.saturating_add(1).max(0) as u32,
        }
    }

    /// The number of tag items this describes.
    fn len(&self) -> usize {
        // The track number is always written.
        1 + [&self.album, &self.artist, &self.title]
            .iter()
            .filter(|value| value.is_some())
            .count()
    }
}

/// Replace every tag of the file at `path` with `tags`.
///
/// Returns `false` if the file already carried exactly these tags and was
/// therefore left untouched.
///
/// Fails if the file is not in an audio format lofty recognizes, if that format
/// cannot store a writable tag, or if the write itself fails.
pub fn write(path: &Path, tags: &AudioTags) -> Result<bool> {
    // The format has to be determined from the content: while a track is being
    // imported its file is still called `<name>.<extension>.part`, so the
    // extension says nothing.
    let probe = Probe::open(path)
        .with_context(|| format!("Failed to open {}", path.display()))?
        .options(
            ParseOptions::new()
                .read_properties(false)
                .read_cover_art(false),
        )
        .guess_file_type()
        .with_context(|| format!("Failed to read {}", path.display()))?;

    let file_type = probe
        .file_type()
        .ok_or_else(|| anyhow!("The file is not in a known audio format"))?;

    let tag_type = file_type.primary_tag_type();

    if !file_type.tag_support(tag_type).is_writable() {
        bail!("Tags cannot be written to {file_type:?} files");
    }

    let existing = probe
        .read()
        .with_context(|| format!("Failed to read {}", path.display()))?;

    if is_up_to_date(&existing, tags, tag_type) {
        return Ok(false);
    }

    let mut tag = Tag::new(tag_type);

    if let Some(album) = &tags.album {
        tag.set_album(album.to_owned());
    }

    if let Some(artist) = &tags.artist {
        tag.set_artist(artist.to_owned());
    }

    if let Some(title) = &tags.title {
        tag.set_title(title.to_owned());
    }

    tag.set_track(tags.track_number);

    // Writing a freshly built tag with `remove_others` is what makes the file
    // carry exactly the tags Musicus manages: anything the source file brought
    // along, in this or in any other tag format, is dropped.
    tag.save_to_path(path, WriteOptions::new().remove_others(true))
        .with_context(|| format!("Failed to write the tags of {}", path.display()))?;

    Ok(true)
}

/// Whether the user turned this tag off by emptying its pattern.
///
/// Unlike a file name, a tag may legitimately be left out, so an empty pattern
/// is a valid configuration rather than an error.
pub fn is_disabled(pattern: &str) -> bool {
    pattern.trim().is_empty()
}

/// Whether `pattern` can be used as a tag pattern.
///
/// The error describes what is wrong with the pattern and is meant to be shown
/// to the user.
pub fn validate(pattern: &str) -> Result<()> {
    preview(pattern).map(|_| ())
}

/// The tag value `pattern` would produce for an example track.
///
/// An empty result means that no tag would be written, which is a valid
/// configuration. This doubles as validation: a pattern that cannot be rendered
/// for the example cannot be rendered for a real track either.
pub fn preview(pattern: &str) -> Result<String> {
    if is_disabled(pattern) {
        return Ok(String::new());
    }

    Ok(tidy(&pattern::render(pattern, &TrackData::example())?))
}

/// Make `value` presentable as a tag value.
///
/// Unlike a file name, a tag stays in its original script: only the characters
/// that would confuse a tag reader and the separators left behind by an empty
/// placeholder are removed.
fn tidy(value: &str) -> String {
    let replaced = value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .collect::<String>();

    pattern::collapse_separators(&replaced, SEPARATORS)
        .trim_matches(|c| SEPARATORS.contains(&c))
        .to_owned()
}

/// Whether the file already carries exactly `tags` and nothing else.
///
/// Rewriting a file that is already correct would change its modification time
/// for nothing, which matters because a reorganization walks the whole library.
fn is_up_to_date(file: &lofty::file::TaggedFile, tags: &AudioTags, tag_type: TagType) -> bool {
    // A second tag in another format would have to be removed, whatever the
    // first one contains.
    let [tag] = file.tags() else {
        return false;
    };

    tag.tag_type() == tag_type
        && tag.len() == tags.len()
        && tag.album().as_deref() == tags.album.as_deref()
        && tag.artist().as_deref() == tags.artist.as_deref()
        && tag.title().as_deref() == tags.title.as_deref()
        && tag.track() == Some(tags.track_number)
}
