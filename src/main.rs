// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis` binary: cross-examines a document and writes a critique report.

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

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
        /// Use the offline echo backend (no API keys, no network). For testing.
        #[arg(long)]
        echo: bool,
        /// RNG seed for provider assignment (roster path). If omitted, a random
        /// seed is generated and printed in the report so the run is
        /// reproducible. Ignored by --echo.
        #[arg(long)]
        seed: Option<u64>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Critique {
            input,
            out,
            echo,
            seed,
        } => match critique(&input, echo, seed) {
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

fn critique(
    input: &std::path::Path,
    echo: bool,
    seed: Option<u64>,
) -> Result<String, praxis::PraxisError> {
    let source = input.to_string_lossy().to_string();
    let text = std::fs::read_to_string(input)
        .map_err(|e| praxis::PraxisError::agent_failure(input.to_string_lossy(), e.to_string()))?;

    if echo {
        return praxis::cli::run_critique_echo(&text, &source);
    }

    #[cfg(feature = "backend-http")]
    {
        // Generate a seed if none given, so every roster run is reproducible.
        let seed = seed.unwrap_or_else(rand::random);
        praxis::cli::run_critique(&text, &source, seed)
    }

    #[cfg(not(feature = "backend-http"))]
    {
        // No HTTP backend compiled in: fall back to echo with a notice.
        let _ = seed;
        let mut report = praxis::cli::run_critique_echo(&text, &source)?;
        report.push_str("\n_(built without `backend-http`; used the echo backend)_\n");
        Ok(report)
    }
}
