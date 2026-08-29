//! Build the pre-made Musicus metadata database from its JSON definitions.
//!
//! The `musicus-data` repository (`metadata.json`, `works.json`) defines the
//! catalogue served to Musicus users. This tool parses those files, checks
//! them against each other, and compiles the result into a metadata database
//! that a Musicus library can use as its downloaded `metadata.musdb`.

mod build;
mod sources;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

/// Parse the Musicus metadata definitions and compile them into a metadata
/// database.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compile the JSON definitions into a metadata database.
    Build(BuildArgs),
    /// Check the JSON definitions and report problems, without writing
    /// anything.
    Check(FileArgs),
}

/// The two JSON definition files. Both default to well-known names inside
/// `data`, but each can be pointed at explicitly.
#[derive(Args, Debug)]
struct FileArgs {
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

impl FileArgs {
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
struct BuildArgs {
    #[command(flatten)]
    files: FileArgs,

    /// Where to write the metadata database. An existing file is replaced.
    #[arg(short, long, default_value = "metadata.musdb")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build(args) => {
            let metadata = sources::read_metadata(&args.files.metadata_path())?;
            let works = sources::read_works(&args.files.works_path())?;

            let summary = build::build(&metadata, &works, &args.output)?;
            print_warnings(&summary.warnings);

            println!(
                "Wrote a metadata database to {}",
                args.output.to_string_lossy()
            );
            print_counts(&summary.summary);
        }
        Command::Check(files) => {
            let metadata = sources::read_metadata(&files.metadata_path())?;
            let works = sources::read_works(&files.works_path())?;

            let validation = build::validate(&metadata, &works)?;
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
        }
    }

    Ok(())
}

/// Print the numbers of a successful build.
fn print_counts(summary: &build::Summary) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_args_default_to_the_data_directory() {
        let args = FileArgs {
            data: PathBuf::from("/tmp/somewhere"),
            metadata: None,
            works: None,
        };

        assert_eq!(
            args.metadata_path(),
            PathBuf::from("/tmp/somewhere/metadata.json")
        );
        assert_eq!(
            args.works_path(),
            PathBuf::from("/tmp/somewhere/works.json")
        );
    }
}
