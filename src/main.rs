// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis` binary: cross-examines a document and writes a critique report.

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use praxis::cli::run_critique;

/// Cross-examine a document with a panel of critic personas.
#[derive(Debug, Parser)]
#[command(name = "praxis", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Critique a document and write a markdown report.
    Critique {
        /// Path to the document to critique (markdown).
        input: PathBuf,
        /// Where to write the critique report. Defaults to stdout.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Critique { input, out } => match critique(&input) {
            Ok(markdown) => {
                match out {
                    Some(path) => {
                        if let Err(e) = std::fs::write(&path, markdown) {
                            eprintln!("failed to write {path:?}: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    None => print!("{markdown}"),
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("praxis: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn critique(input: &std::path::Path) -> Result<String, praxis::PraxisError> {
    let source = input.to_string_lossy().to_string();
    let text = std::fs::read_to_string(input)
        .map_err(|e| praxis::PraxisError::agent_failure(input.to_string_lossy(), e.to_string()))?;
    run_critique(&text, &source)
}
