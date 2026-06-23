// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `praxis` binary: cross-examines a document and writes a critique report.

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Cross-examine a document with a panel of critic personas.
#[derive(Debug, Parser)]
#[command(
    name = "praxis",
    version,
    about = "Multi-agent critique and cross-examination pipeline",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Critique a document and write a critique report.
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
        /// Path to a credentials config file (overrides discovery). Roster path only.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Emit the report as JSON (machine-readable) instead of markdown.
        /// When set, errors are also emitted as structured JSON on stderr and
        /// the exit code follows the Praxis scheme (see `praxis capabilities`).
        #[arg(long)]
        json: bool,
        /// Resolve the roster and emit a run plan (JSON) without making any API
        /// calls. Lets an agent verify intent before spending tokens.
        #[arg(long)]
        dry_run: bool,
    },
    /// Print Praxis's capabilities as JSON: version, subcommands, providers
    /// (and which are currently authed), personas, topologies, exit codes.
    ///
    /// Designed for AI-agent discoverability.
    Capabilities,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Capabilities => {
            #[cfg(feature = "backend-http")]
            {
                let caps = praxis::Capabilities::with_current_auth();
                let json = serde_json::to_string_pretty(&caps).unwrap_or_else(|_| "{}".to_owned());
                println!("{json}");
                ExitCode::SUCCESS
            }
            #[cfg(not(feature = "backend-http"))]
            {
                let caps = praxis::Capabilities::static_info();
                let json = serde_json::to_string_pretty(&caps).unwrap_or_else(|_| "{}".to_owned());
                println!("{json}");
                ExitCode::SUCCESS
            }
        }
        Command::Critique {
            input,
            out,
            echo,
            seed,
            config,
            json,
            dry_run,
        } => match run(&input, echo, seed, config.as_deref(), json, dry_run) {
            Ok(output) => {
                match out {
                    Some(path) => {
                        if let Err(e) = std::fs::write(&path, output) {
                            eprintln!("failed to write {path:?}: {e}");
                            return ExitCode::FAILURE;
                        }
                    }
                    None => print!("{output}"),
                }
                ExitCode::SUCCESS
            }
            Err(err) => {
                emit_error(&err, json);
                ExitCode::from(err.exit_code())
            }
        },
    }
}

/// Emits an error appropriately: structured JSON on stderr if `--json`, else
/// prose.
fn emit_error(err: &praxis::PraxisError, json: bool) {
    if json {
        #[cfg(feature = "json")]
        {
            eprintln!("{}", err.to_error_json());
            return;
        }
    }
    eprintln!("praxis: {err}");
}

fn run(
    input: &std::path::Path,
    echo: bool,
    seed: Option<u64>,
    config: Option<&std::path::Path>,
    json: bool,
    dry_run: bool,
) -> Result<String, praxis::PraxisError> {
    let source = input.to_string_lossy().to_string();
    let text = std::fs::read_to_string(input)
        .map_err(|e| praxis::PraxisError::agent_failure(input.to_string_lossy(), e.to_string()))?;

    if echo {
        return praxis::cli::run_critique_echo(&text, &source);
    }

    #[cfg(feature = "backend-http")]
    {
        let seed = seed.unwrap_or_else(rand::random);
        if dry_run {
            return praxis::cli::plan_critique(&text, &source, seed, config, json);
        }
        praxis::cli::run_critique(&text, &source, seed, config, json)
    }

    #[cfg(not(feature = "backend-http"))]
    {
        let _ = (seed, config, json, dry_run);
        let mut report = praxis::cli::run_critique_echo(&text, &source)?;
        report.push_str("\n_(built without `backend-http`; used the echo backend)_\n");
        Ok(report)
    }
}
