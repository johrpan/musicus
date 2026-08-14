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

/// The smallest file that lofty accepts as audio.
///
/// Tests need a real container rather than arbitrary bytes. WAV is the cheapest
/// one to build by hand, and it is tagged with ID3v2 like MP3, so it exercises
/// the same write path.
#[cfg(test)]
pub(crate) fn minimal_wav() -> Vec<u8> {
    const SAMPLES: &[u8] = &[0; 8];

    let mut file = Vec::new();

    file.extend_from_slice(b"RIFF");
    file.extend_from_slice(&(36u32 + SAMPLES.len() as u32).to_le_bytes());
    file.extend_from_slice(b"WAVE");

    file.extend_from_slice(b"fmt ");
    file.extend_from_slice(&16u32.to_le_bytes()); // Chunk size.
    file.extend_from_slice(&1u16.to_le_bytes()); // Uncompressed PCM.
    file.extend_from_slice(&1u16.to_le_bytes()); // One channel.
    file.extend_from_slice(&44100u32.to_le_bytes()); // Sample rate.
    file.extend_from_slice(&88200u32.to_le_bytes()); // Bytes per second.
    file.extend_from_slice(&2u16.to_le_bytes()); // Block align.
    file.extend_from_slice(&16u16.to_le_bytes()); // Bits per sample.

    file.extend_from_slice(b"data");
    file.extend_from_slice(&(SAMPLES.len() as u32).to_le_bytes());
    file.extend_from_slice(SAMPLES);

    file
}

#[cfg(test)]
mod tests {
    use std::fs;

    use lofty::probe::read_from_path;
    use tempfile::TempDir;

    use super::*;

    fn data() -> TrackData {
        TrackData {
            composer: "Antonín Dvořák".to_owned(),
            work: "Symfonie č. 9".to_owned(),
            part: "Largo".to_owned(),
            performers: "Berliner Philharmoniker".to_owned(),
            index: "02".to_owned(),
            year: "1993".to_owned(),
        }
    }

    fn wav(dir: &TempDir, name: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, minimal_wav()).unwrap();
        path
    }

    #[test]
    fn default_patterns_describe_the_track() {
        let tags = AudioTags::render(&Patterns::default(), &data(), 1);

        assert_eq!(tags.album.as_deref(), Some("Antonín Dvořák: Symfonie č. 9"));
        assert_eq!(tags.artist.as_deref(), Some("Berliner Philharmoniker"));
        assert_eq!(tags.title.as_deref(), Some("Largo"));
        assert_eq!(tags.track_number, 2);
    }

    #[test]
    fn tag_values_keep_their_script() {
        let tags = AudioTags::render(&Patterns::default(), &data(), 0);

        assert!(tags.album.unwrap().contains("Dvořák"));
    }

    #[test]
    fn an_empty_value_does_not_leave_its_separator_behind() {
        let mut data = data();
        data.composer = String::new();

        let tags = AudioTags::render(&Patterns::default(), &data, 0);

        assert_eq!(tags.album.as_deref(), Some("Symfonie č. 9"));
    }

    #[test]
    fn a_track_without_parts_is_titled_after_the_work() {
        let mut data = data();
        data.part = String::new();

        let tags = AudioTags::render(&Patterns::default(), &data, 0);

        assert_eq!(tags.title.as_deref(), Some("Symfonie č. 9"));
    }

    #[test]
    fn a_value_that_cannot_be_rendered_is_left_out() {
        let patterns = Patterns {
            album: "{bogus}".to_owned(),
            ..Patterns::default()
        };

        assert!(AudioTags::render(&patterns, &data(), 0).album.is_none());
    }

    #[test]
    fn unusable_patterns_are_rejected() {
        assert!(validate("{bogus}").is_err());
        assert!(validate("{composer").is_err());
        assert!(validate(DEFAULT_ALBUM_PATTERN).is_ok());
        assert_eq!(
            preview(DEFAULT_ALBUM_PATTERN).unwrap(),
            "Ludwig van Beethoven: Symphony No. 5 in C minor, Op. 67"
        );
    }

    /// A tag the user does not want is configured by emptying its pattern.
    #[test]
    fn an_empty_pattern_writes_no_tag() {
        assert!(validate("").is_ok());
        assert!(preview("  ").unwrap().is_empty());

        let patterns = Patterns {
            album: String::new(),
            title: String::new(),
            ..Patterns::default()
        };

        let tags = AudioTags::render(&patterns, &data(), 0);

        assert!(tags.album.is_none());
        assert!(tags.title.is_none(), "the fallback must not override it");
        assert_eq!(tags.artist.as_deref(), Some("Berliner Philharmoniker"));
    }

    #[test]
    fn tags_can_be_written_and_read_back() {
        let dir = TempDir::new().unwrap();
        let path = wav(&dir, "track.wav");
        let tags = AudioTags::render(&Patterns::default(), &data(), 1);

        assert!(write(&path, &tags).unwrap());

        let file = read_from_path(&path).unwrap();
        let tag = file.primary_tag().unwrap();

        assert_eq!(
            tag.album().as_deref(),
            Some("Antonín Dvořák: Symfonie č. 9")
        );
        assert_eq!(tag.artist().as_deref(), Some("Berliner Philharmoniker"));
        assert_eq!(tag.title().as_deref(), Some("Largo"));
        assert_eq!(tag.track(), Some(2));
    }

    #[test]
    fn tags_of_the_source_file_do_not_survive() {
        let dir = TempDir::new().unwrap();
        let path = wav(&dir, "track.wav");

        let mut previous = Tag::new(TagType::Id3v2);
        previous.set_comment("Ripped by somebody".to_owned());
        previous.set_genre("Rock".to_owned());
        previous.set_album("Greatest Hits".to_owned());
        previous.save_to_path(&path, WriteOptions::new()).unwrap();

        let tags = AudioTags::render(&Patterns::default(), &data(), 0);
        assert!(write(&path, &tags).unwrap());

        let file = read_from_path(&path).unwrap();
        let tag = file.primary_tag().unwrap();

        assert_eq!(tag.comment(), None);
        assert_eq!(tag.genre(), None);
        assert_eq!(
            tag.album().as_deref(),
            Some("Antonín Dvořák: Symfonie č. 9")
        );
        assert_eq!(file.tags().len(), 1);
    }

    #[test]
    fn a_file_that_is_already_correct_is_not_rewritten() {
        let dir = TempDir::new().unwrap();
        let path = wav(&dir, "track.wav");
        let tags = AudioTags::render(&Patterns::default(), &data(), 0);

        assert!(write(&path, &tags).unwrap());

        let written = fs::read(&path).unwrap();

        assert!(!write(&path, &tags).unwrap());
        assert_eq!(fs::read(&path).unwrap(), written);
    }

    #[test]
    fn a_file_that_is_not_audio_is_rejected_and_left_alone() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("track.mp3");
        fs::write(&path, b"not audio at all").unwrap();

        let tags = AudioTags::render(&Patterns::default(), &data(), 0);

        assert!(write(&path, &tags).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not audio at all");
    }
}
