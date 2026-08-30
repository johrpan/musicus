//! CLI for building Musicus metadata catalogues and libraries from JSON.
//!
//! All parsing, validation and database-writing logic lives in
//! `musicus-library` (`musicus_library::catalog` for the metadata catalogue,
//! `musicus_library::catalog_import` for a library's recordings and tracks);
//! this crate only reads files, calls that logic and reports the result.

use std::{
    fs::{self, File},
    io::BufReader,
    ops::ControlFlow,
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use musicus_library::{
    catalog::{self, MetadataFile, WorksFile},
    catalog_import::{self, RecordingsFile},
    exchange::{self, ExchangeEvent},
    Library,
};
use tempfile::TempDir;

/// Build Musicus metadata catalogues (`.musdb`) and libraries (`.muslib`)
/// from JSON definitions.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compile a metadata catalogue's JSON definitions into a `.musdb`.
    BuildMetadata(BuildMetadataArgs),
    /// Check a metadata catalogue's JSON definitions and report problems,
    /// without writing anything.
    CheckMetadata(MetadataFileArgs),
    /// Build a library's `.muslib` from a recordings JSON file, resolving
    /// every catalogue reference against a metadata database.
    BuildLibrary(BuildLibraryArgs),
}

/// The two JSON definition files of a metadata catalogue. Both default to
/// well-known names inside `data`, but each can be pointed at explicitly.
#[derive(Args, Debug)]
struct MetadataFileArgs {
    /// Directory containing metadata.json and works.json.
    #[arg(long, default_value = ".")]
    data: PathBuf,

    /// Path to metadata.json [default: <data>/metadata.json]
    #[arg(long)]
    metadata: Option<PathBuf>,

    /// Path to works.json [default: <data>/works.json]
    #[arg(long)]
    works: Option<PathBuf>,
}

impl MetadataFileArgs {
    fn metadata_path(&self) -> PathBuf {
        self.metadata
            .clone()
            .unwrap_or_else(|| self.data.join("metadata.json"))
    }

    fn works_path(&self) -> PathBuf {
        self.works
            .clone()
            .unwrap_or_else(|| self.data.join("works.json"))
    }
}

#[derive(Args, Debug)]
struct BuildMetadataArgs {
    #[command(flatten)]
    files: MetadataFileArgs,

    /// Where to write the metadata database. An existing file is replaced.
    #[arg(short, long, default_value = "metadata.musdb")]
    output: PathBuf,
}

#[derive(Args, Debug)]
struct BuildLibraryArgs {
    /// Path to the metadata database to resolve catalogue references
    /// against.
    #[arg(long)]
    metadata: PathBuf,

    /// Path to the recordings JSON file.
    #[arg(long, default_value = "recordings.json")]
    recordings: PathBuf,

    /// Directory each track's relative file path is resolved against.
    #[arg(long, default_value = ".")]
    tracks_dir: PathBuf,

    /// Where to write the library archive. An existing file is replaced.
    #[arg(short, long, default_value = "library.muslib")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::BuildMetadata(args) => build_metadata(args),
        Command::CheckMetadata(files) => check_metadata(files),
        Command::BuildLibrary(args) => build_library(args),
    }
}

fn build_metadata(args: BuildMetadataArgs) -> Result<()> {
    let metadata = read_json::<MetadataFile>(&args.files.metadata_path())?;
    let works = read_json::<WorksFile>(&args.files.works_path())?;

    let output = catalog::build(&metadata, &works, &args.output)?;
    print_warnings(&output.warnings);

    println!(
        "Wrote a metadata database to {}",
        args.output.to_string_lossy()
    );
    print_metadata_summary(&output.summary);

    Ok(())
}

fn check_metadata(files: MetadataFileArgs) -> Result<()> {
    let metadata = read_json::<MetadataFile>(&files.metadata_path())?;
    let works = read_json::<WorksFile>(&files.works_path())?;

    let validation = catalog::validate(&metadata, &works)?;
    print_warnings(&validation.warnings);

    println!("The metadata definitions are valid.");
    println!("  persons:     {}", validation.persons.len());
    println!("  roles:       {}", validation.roles.len());
    println!("  instruments: {}", validation.instruments.len());
    println!("  ensembles:   {}", validation.ensembles.len());
    println!("  tags:        {}", validation.tag_takes_value.len());
    println!("  works:       {}", validation.work_keys.len());

    if !validation.warnings.is_empty() {
        println!(
            "{} problem{} found.",
            validation.warnings.len(),
            if validation.warnings.len() == 1 {
                ""
            } else {
                "s"
            }
        );
    }

    Ok(())
}

fn build_library(args: BuildLibraryArgs) -> Result<()> {
    let recordings = read_json::<RecordingsFile>(&args.recordings)?;

    let library_dir = TempDir::new().context("Failed to create a scratch library directory")?;
    let cache_dir = TempDir::new().context("Failed to create a metadata cache directory")?;
    fs::create_dir_all(library_dir.path())
        .context("Failed to create the scratch library directory")?;

    let library = Library::new(library_dir.path(), cache_dir.path())
        .context("Failed to create the scratch library")?;

    let summary = catalog_import::import(
        &library,
        &args.metadata,
        &args.tracks_dir,
        &recordings,
        &mut observe,
    )
    .context("Failed to import the recordings")?;

    exchange::export_library_zip(&library.exchange_context(), &args.output, &mut observe)
        .context("Failed to write the library archive")?;

    println!("Wrote a library to {}", args.output.to_string_lossy());
    println!("  recordings: {}", summary.recordings);
    println!("  tracks:     {}", summary.tracks);

    Ok(())
}

/// Report exchange progress on the terminal; a one-shot CLI run never
/// cancels an operation.
fn observe(event: ExchangeEvent) -> ControlFlow<()> {
    match event {
        ExchangeEvent::Message(message) => println!("{message}"),
        ExchangeEvent::Warning(warning) => eprintln!("warning: {warning}"),
        ExchangeEvent::Progress(_) => {}
    }

    ControlFlow::Continue(())
}

fn read_json<T: for<'de> serde::Deserialize<'de>>(path: &PathBuf) -> Result<T> {
    let file = File::open(path).with_context(|| format!("Failed to open {}", path.display()))?;
    serde_json::from_reader(BufReader::new(file))
        .with_context(|| format!("Failed to parse {}", path.display()))
}

/// Print the numbers of a successful metadata catalogue build.
fn print_metadata_summary(summary: &catalog::Summary) {
    println!("  persons:           {}", summary.persons);
    println!("  roles:             {}", summary.roles);
    println!("  instruments:       {}", summary.instruments);
    println!("  ensembles:         {}", summary.ensembles);
    println!("  tags:              {}", summary.tags);
    println!("  works:             {}", summary.works);
    println!("  composer credits:  {}", summary.composer_credits);
    println!("  tag associations:  {}", summary.tag_associations);
    println!("  instrumentations:  {}", summary.instrumentations);
    println!("  derived works:     {}", summary.derived);
}

/// Print validation warnings to stderr.
fn print_warnings(warnings: &[String]) {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
}
