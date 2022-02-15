use std::path::PathBuf;

use anyhow::{bail, ensure, Result};
use clap::{AppSettings, IntoApp, Parser};

use crate::params::{ParamNames, Params};

#[derive(Parser)]
#[clap(global_setting(AppSettings::AllowNegativeNumbers))]
struct Cli {
    /// CSV/TSV files to consolidate (at least two).
    #[clap(short, long, parse(from_os_str))]
    #[structopt(required = true, min_values = 2)]
    inputs: Vec<PathBuf>,

    /// Path to the consolidated CSV/TSV file (must be different from all
    /// the input files; will be overridden if exists).
    #[clap(short, long, parse(from_os_str))]
    output: PathBuf,

    /// Delimiter character.
    #[clap(short, long, default_value_t = '\t')]
    delimiter: char,

    /// Indices of columns containing data that the records should be identified by
    /// (1-based; use positive values to refer to columns left-to-right, negative
    /// values to refer to columns right-to-left, zero to refer to a special column
    /// whose values are considered unique for each individual record).
    #[clap(short, long)]
    common: Vec<i32>,

    /// Allow consolidation when all the input files contain a single column.
    #[clap(long)]
    single: bool,

    /// Still allow consolidation when there are multiple ways to merge records.
    #[clap(long)]
    multi: bool,

    /// Filler string for output cells with otherwise missing values (which would
    /// occur for records missing from some of the input files but present in others).
    #[clap(long)]
    filler: Option<String>,

    /// Amount of matching data in common columns that should trigger a warning if it's
    /// not a full match (e.g., "8;ab" vs. "8;ac" would count as 1.5 matching columns).
    #[clap(long)]
    warn_similar: Option<f64>,

    /// Warn about any unmatched records.
    #[clap(long)]
    warn_unmatched: bool,
}

macro_rules! argument_name {
    ($app:expr, $struct:ident.$field:ident) => {
        {
            let _ = $struct.$field;
            let name = stringify!($field);
            let result = $app.get_arguments()
                .find(|arg| arg.get_name() == name)
                .and_then(|arg| Some(format!("{arg}")));
            result
        }
    };
}

fn check_convert_delimiter(delimiter: char) -> Result<u8> {
    ensure!(delimiter.is_ascii(), "'{delimiter}' is not an ASCII character; \
                                   only ASCII delimiters are currently supported.");
    Ok(delimiter as u8)
}

fn check_similarity_warn_level(similarity: Option<f64>, common_columns: &[i32]) -> Result<()> {
    let column_count = common_columns.len();
    match similarity {
        Some(level) if level <= 0.0 =>
            bail!("Similarity warn level must be positive, got {level}."),
        Some(level) if level >= column_count as f64 =>
            bail!("Similarity warn level must be less than the number of common columns, \
                   got {level} >= {column_count}."),
        _ => Ok(()),
    }
}

fn check_inputs(inputs: &[PathBuf], output: &PathBuf) -> Result<()> {
    for input in inputs {
        ensure!(input.exists(), "{} does not exist.", input.display());
        ensure!(input.is_file(), "{} is not a file.", input.display());
        ensure!(input != output, "{} is used both as an input and as the output.", input.display());
    }
    Ok(())
}

fn convert_filler(filler: Option<String>) -> String {
    filler.unwrap_or(String::from(""))
}

pub fn get_params() -> Result<Params> {
    let cli: Cli = Cli::parse();
    let app = Cli::into_app();
    let delimiter = check_convert_delimiter(cli.delimiter)?;
    check_similarity_warn_level(cli.warn_similar, &cli.common)?;
    check_inputs(&cli.inputs, &cli.output)?;
    let filler = convert_filler(cli.filler);
    Ok(Params {
        inputs: cli.inputs,
        output: cli.output,
        delimiter,
        common_columns: cli.common,
        allow_single_column: cli.single,
        allow_multi_merge: cli.multi,
        filler,
        similarity_warn_level: cli.warn_similar,
        warn_unmatched: cli.warn_unmatched,
        names: ParamNames {
            allow_single_column: argument_name!(app, cli.single).unwrap(),
            allow_multi_merge: argument_name!(app, cli.multi).unwrap(),
        }
    })
}
