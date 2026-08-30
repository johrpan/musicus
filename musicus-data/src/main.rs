//! CLI for building Musicus metadata catalogues and libraries from JSON.
//!
//! All parsing, validation and database-writing logic lives in
//! `musicus-library` (`musicus_library::catalog` for the source format,
//! `musicus_library::catalog_import` for a library's recordings and tracks);
//! this crate only reads files, calls that logic and reports the result.
//!
//! There is no format distinction between the files given to a command: each
//! is parsed as a `musicus_library::catalog::Definitions` and they are all
//! folded together before use, so the same `persons`/`works`/`recordings`
//! content can be split across any number of files however is convenient.

use std::{
    fs::{self, File},
    io::BufReader,
    ops::ControlFlow,
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use musicus_library::{
    catalog::{self, Definitions},
    catalog_import,
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
    /// Compile the catalogue entities of the given files into a `.musdb`,
    /// ignoring any recordings.
    BuildMetadata(BuildMetadataArgs),
    /// Check the catalogue entities of the given files and report problems,
    /// without writing anything.
    CheckMetadata(SourceArgs),
    /// Build a library's `.muslib` from the recordings of the given files,
    /// importing every catalogue entity a recording needs and ignoring
    /// everything else.
    BuildLibrary(BuildLibraryArgs),
}

/// One or more source files, each parsed as a [`Definitions`] and folded
/// together.
#[derive(Args, Debug)]
struct SourceArgs {
    /// The JSON source files.
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

impl SourceArgs {
    fn read(&self) -> Result<Definitions> {
        let sources = self
            .files
            .iter()
            .map(read_json::<Definitions>)
            .collect::<Result<Vec<_>>>()?;

        Ok(Definitions::merge(sources))
    }
}

#[derive(Args, Debug)]
struct BuildMetadataArgs {
    #[command(flatten)]
    sources: SourceArgs,

    /// Where to write the metadata database. An existing file is replaced.
    #[arg(short, long, default_value = "metadata.musdb")]
    output: PathBuf,
}

#[derive(Args, Debug)]
struct BuildLibraryArgs {
    #[command(flatten)]
    sources: SourceArgs,

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
        Command::CheckMetadata(sources) => check_metadata(sources),
        Command::BuildLibrary(args) => build_library(args),
    }
}

fn build_metadata(args: BuildMetadataArgs) -> Result<()> {
    let definitions = args.sources.read()?;

    let output = catalog::build(&definitions, &args.output)?;
    print_warnings(&output.warnings);

    println!(
        "Wrote a metadata database to {}",
        args.output.to_string_lossy()
    );
    print_metadata_summary(&output.summary);

    Ok(())
}

fn check_metadata(sources: SourceArgs) -> Result<()> {
    let definitions = sources.read()?;

    let validation = catalog::validate(&definitions)?;
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
    let definitions = args.sources.read()?;

    // Recordings resolve their catalogue references against a real metadata
    // database, so everything the sources define is compiled into a scratch
    // one first; only what a recording actually reaches ends up in the
    // library.
    let metadata_dir = TempDir::new().context("Failed to create a scratch metadata directory")?;
    let metadata_path = metadata_dir.path().join("metadata.musdb");
    catalog::build(&definitions, &metadata_path)
        .context("Failed to build the metadata catalogue")?;

    let library_dir = TempDir::new().context("Failed to create a scratch library directory")?;
    let cache_dir = TempDir::new().context("Failed to create a metadata cache directory")?;
    fs::create_dir_all(library_dir.path())
        .context("Failed to create the scratch library directory")?;

    let library = Library::new(library_dir.path(), cache_dir.path())
        .context("Failed to create the scratch library")?;

    let summary = catalog_import::import(
        &library,
        &metadata_path,
        &args.tracks_dir,
        &definitions.recordings,
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
