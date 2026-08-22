//! Generator for exemplary Musicus libraries.
//!
//! Writes a complete library folder — `musicus.musdb` plus track files — filled
//! with procedurally generated metadata, for testing search, browsing and the
//! other library features at a realistic scale.

mod generate;
mod names;

use std::{fs, io::Write, path::PathBuf};

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::generate::Counts;

/// Generate an exemplary Musicus library with realistic looking data.
///
/// All names are invented; they are assembled from word tables rather than
/// taken from real people or catalogues. Track files are placeholders, not
/// audio: the generated library is complete and browsable, but not playable.
///
/// The same seed always produces the same names and structure. Entity IDs and
/// timestamps are minted by the library itself, so `musicus.musdb` is not
/// byte-identical between two runs.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Folder to write the library to. Created if it does not exist.
    #[arg(short, long)]
    output: PathBuf,

    /// Seed for the random number generator.
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Number of recordings. The other counts default to a share of this.
    #[arg(short, long, default_value_t = 200)]
    recordings: usize,

    /// Number of works [default: half the recordings]
    #[arg(long)]
    works: Option<usize>,

    /// Number of persons [default: a third of the works]
    #[arg(long)]
    persons: Option<usize>,

    /// Number of ensembles [default: a quarter of the persons]
    #[arg(long)]
    ensembles: Option<usize>,

    /// Number of instruments.
    #[arg(long, default_value_t = 20)]
    instruments: usize,

    /// Number of performer roles.
    #[arg(long, default_value_t = 6)]
    roles: usize,

    /// Number of tracks to import per recording.
    #[arg(long, default_value_t = 1)]
    tracks_per_recording: usize,

    /// Generate into a folder that is not empty.
    #[arg(short, long)]
    force: bool,
}

impl Args {
    /// The counts to generate, filling in the derived defaults.
    ///
    /// Every kind gets at least one entity as long as recordings were asked
    /// for, so that a tiny library is still fully wired up.
    fn counts(&self) -> Counts {
        let works = self.works.unwrap_or_else(|| at_least_one(self.recordings / 2));
        let persons = self.persons.unwrap_or_else(|| at_least_one(works / 3));
        let ensembles = self.ensembles.unwrap_or_else(|| at_least_one(persons / 4));

        Counts {
            instruments: self.instruments,
            roles: self.roles,
            persons,
            ensembles,
            works,
            recordings: self.recordings,
            tracks_per_recording: self.tracks_per_recording,
        }
    }
}

fn at_least_one(count: usize) -> usize {
    count.max(1)
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.output.exists() && !args.force {
        let is_empty = fs::read_dir(&args.output)
            .with_context(|| format!("Failed to read {}", args.output.display()))?
            .next()
            .is_none();

        if !is_empty {
            bail!(
                "{} is not empty. Pass --force to generate into it anyway.",
                args.output.display()
            );
        }
    }

    let counts = args.counts();
    let summary = generate::generate(&args.output, args.seed, counts, progress)?;

    // The progress line is overwritten in place, so it needs terminating.
    println!();
    println!("Generated a library at {}", args.output.display());
    println!("  seed:        {}", args.seed);
    println!("  persons:     {}", summary.persons);
    println!("  instruments: {}", summary.instruments);
    println!("  roles:       {}", summary.roles);
    println!("  ensembles:   {}", summary.ensembles);
    println!("  works:       {}", summary.works);
    println!("  recordings:  {}", summary.recordings);
    println!("  tracks:      {}", summary.tracks);
    println!();
    println!("The track files are placeholders, so the library is not playable.");

    Ok(())
}

/// Report track import progress on a single, rewritten line.
///
/// Importing copies a file and opens a transaction per track, so a few hundred
/// recordings take long enough to warrant feedback. A failure to write to
/// stdout is not worth aborting a generation run over.
fn progress(done: usize, total: usize) {
    print!("\rImporting tracks: {done}/{total} recordings");
    let _ = std::io::stdout().flush();
}
