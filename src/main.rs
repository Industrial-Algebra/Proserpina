// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! The `proserpina` binary: cross-examines a document and writes a critique report.

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Cross-examine a document with a panel of critic personas.
#[derive(Debug, Parser)]
#[command(
    name = "proserpina",
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
        /// the exit code follows the Proserpina scheme (see `proserpina capabilities`).
        #[arg(long)]
        json: bool,
        /// Resolve the roster and emit a run plan (JSON) without making any API
        /// calls. Lets an agent verify intent before spending tokens.
        #[arg(long)]
        dry_run: bool,
        /// Panel name to use (built-in: default/duo/panel, or a [panels.NAME]
        /// section from the config). Defaults to "default".
        #[arg(long)]
        panel: Option<String>,
        /// Override the retry policy's max attempts (CLI > [retry] config >
        /// default 3).
        #[arg(long)]
        max_attempts: Option<u32>,
        /// Override the retry policy's per-attempt timeout in seconds (CLI >
        /// [retry] config > default 60).
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Print Proserpina's capabilities as JSON: version, subcommands, providers
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
                let caps = proserpina::Capabilities::with_current_auth();
                let json = serde_json::to_string_pretty(&caps).unwrap_or_else(|_| "{}".to_owned());
                println!("{json}");
                ExitCode::SUCCESS
            }
            #[cfg(not(feature = "backend-http"))]
            {
                let caps = proserpina::Capabilities::static_info();
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
            panel,
            max_attempts,
            timeout,
        } => match run(
            &input,
            echo,
            seed,
            config.as_deref(),
            json,
            dry_run,
            panel.as_deref(),
            max_attempts,
            timeout,
        ) {
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
fn emit_error(err: &proserpina::ProserpinaError, json: bool) {
    if json {
        #[cfg(feature = "json")]
        {
            eprintln!("{}", err.to_error_json());
            return;
        }
    }
    eprintln!("proserpina: {err}");
}

// CLI flag sprawl is inherent to a subcommand entry point; grouping into a
// struct would be over-engineering here.
#[allow(clippy::too_many_arguments)]
fn run(
    input: &std::path::Path,
    echo: bool,
    seed: Option<u64>,
    config: Option<&std::path::Path>,
    json: bool,
    dry_run: bool,
    panel: Option<&str>,
    max_attempts: Option<u32>,
    timeout: Option<u64>,
) -> Result<String, proserpina::ProserpinaError> {
    let source = input.to_string_lossy().to_string();
    let text = std::fs::read_to_string(input).map_err(|e| {
        proserpina::ProserpinaError::agent_failure(input.to_string_lossy(), e.to_string())
    })?;

    if echo {
        return proserpina::cli::run_critique_echo(&text, &source);
    }

    #[cfg(feature = "backend-http")]
    {
        let seed = seed.unwrap_or_else(rand::random);
        // Resolve retry policy: CLI flags > [retry] config > default.
        let retry_config = proserpina::backend::credentials::Credentials::discover_or(config)
            .map(|c| c.retry().clone())
            .unwrap_or_default();
        let policy =
            proserpina::backend::http::RetryPolicy::resolve(&retry_config, max_attempts, timeout);
        if dry_run {
            return proserpina::cli::plan_critique(&text, &source, seed, config, json, panel);
        }
        proserpina::cli::run_critique(&text, &source, seed, config, json, panel, policy)
    }

    #[cfg(not(feature = "backend-http"))]
    {
        let _ = (seed, config, json, dry_run, panel, max_attempts, timeout);
        let mut report = proserpina::cli::run_critique_echo(&text, &source)?;
        report.push_str("\n_(built without `backend-http`; used the echo backend)_\n");
        Ok(report)
    }
}
