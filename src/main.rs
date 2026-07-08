// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

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
    long_about = "Cross-examine a document with a panel of LLM-backed critic personas.\n\n\
                  Quick start:\n  \
                    proserpina critique doc.md --panel panel\n\n\
                  Validate your keys:\n  \
                    proserpina auth check\n"
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
        /// RNG seed for provider assignment. If omitted, a random seed is
        /// generated and printed in the report so the run is reproducible.
        #[arg(long)]
        seed: Option<u64>,
        /// Path to a credentials config file (overrides discovery).
        #[arg(long)]
        config: Option<PathBuf>,
        /// Emit the report as JSON (machine-readable). Also makes errors and
        /// capabilities JSON on stderr/stdout.
        #[arg(long)]
        json: bool,
        /// Resolve the roster and emit a run plan without making any API calls.
        #[arg(long)]
        dry_run: bool,
        /// Panel name: built-in (default/duo/panel) or a [panels.NAME] section.
        /// Use `proserpina panels` to list available panels.
        #[arg(long)]
        panel: Option<String>,
        /// Override the retry policy's max attempts.
        #[arg(long)]
        max_attempts: Option<u32>,
        /// Override the retry policy's per-attempt timeout in seconds.
        #[arg(long)]
        timeout: Option<u64>,
        /// Output language for critiques and findings (e.g. "Japanese", "français").
        /// If omitted, the model chooses based on the document.
        #[arg(long)]
        language: Option<String>,
        /// Comma-separated model names to exclude from the provider pool
        /// (e.g. `--exclude qwen3.7-max,mercury-2`). Also settable via
        /// `exclude = [...]` in credentials.toml.
        #[arg(long)]
        exclude: Option<String>,
    },

    /// Show capabilities: version, providers (and which are authed), panels,
    /// topologies, exit codes. Human-readable by default; --json for machine.
    Capabilities {
        /// Emit as JSON (for AI agents).
        #[arg(long)]
        json: bool,
    },

    /// Check which provider keys are set and actually work.
    Auth {
        #[command(subcommand)]
        action: Option<AuthAction>,
    },

    /// List available critic panels (built-in + config-defined).
    Panels {
        /// Path to a credentials config file (overrides discovery).
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum AuthAction {
    /// Validate each authed provider's key by making a test API call.
    Check {
        /// Path to a credentials config file (overrides discovery).
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// List which providers have keys set (without making API calls).
    List {
        /// Path to a credentials config file (overrides discovery).
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Interactively authenticate with a provider (acquire + store a credential).
    Login {
        /// Provider name (e.g. deepseek, openai, zai). If omitted, shows a
        /// selector.
        provider: Option<String>,
        /// Override the default model for this provider (stored in
        /// credentials.toml). E.g. `--model gpt-5.5`.
        #[arg(long)]
        model: Option<String>,
    },
    /// Remove stored credentials for a provider.
    Logout {
        /// Provider name to remove.
        provider: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Capabilities { json } => cmd_capabilities(json),
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
            language,
            exclude,
        } => match run_critique_cmd(
            &input,
            echo,
            seed,
            config.as_deref(),
            json,
            dry_run,
            panel.as_deref(),
            max_attempts,
            timeout,
            language.as_deref(),
            exclude.as_deref(),
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
        Command::Auth { action } => match action.unwrap_or(AuthAction::Check { config: None }) {
            AuthAction::Check { config } => cmd_auth_check(config.as_deref()),
            AuthAction::List { config } => cmd_auth_list(config.as_deref()),
            AuthAction::Login { provider, model } => {
                cmd_auth_login(provider.as_deref(), model.as_deref())
            }
            AuthAction::Logout { provider } => cmd_auth_logout(&provider),
        },
        Command::Panels { config } => cmd_panels(config.as_deref()),
    }
}

// ---- Subcommand implementations ----

fn cmd_capabilities(json: bool) -> ExitCode {
    #[cfg(feature = "backend-http")]
    {
        let caps = proserpina::Capabilities::with_current_auth();
        if json {
            let j = serde_json::to_string_pretty(&caps).unwrap_or_else(|_| "{}".to_owned());
            println!("{j}");
        } else {
            print_human_capabilities(&caps);
        }
        ExitCode::SUCCESS
    }
    #[cfg(not(feature = "backend-http"))]
    {
        let caps = proserpina::Capabilities::static_info();
        if json {
            let j = serde_json::to_string_pretty(&caps).unwrap_or_else(|_| "{}".to_owned());
            println!("{j}");
        } else {
            print_human_capabilities(&caps);
        }
        ExitCode::SUCCESS
    }
}

fn print_human_capabilities(caps: &proserpina::Capabilities) {
    println!("Proserpina v{}", caps.version);
    println!();
    println!("Providers:");
    for p in &caps.providers {
        let status = if p.authed { "✓ authed" } else { "  —" };
        println!("  {status}  {} ({})", p.name, p.model);
    }
    println!();
    println!("Panels: {}", caps.panels.join(", "));
    println!("Topologies: {}", caps.topologies.join(", "));
    println!("Output: {}", caps.output_formats.join(", "));
}

#[cfg(feature = "backend-http")]
fn cmd_auth_check(config: Option<&std::path::Path>) -> ExitCode {
    use proserpina::backend::credentials::authed_configs_with;
    use proserpina::backend::http::{validate_provider, RetryPolicy};

    let configs = match authed_configs_with(config) {
        Ok(c) if !c.is_empty() => c,
        Ok(_) => {
            eprintln!("No provider keys found. Set DEEPSEEK_API_KEY (or another provider key)");
            eprintln!("or create ~/.config/proserpina/credentials.toml.");
            return ExitCode::from(10);
        }
        Err(e) => {
            eprintln!("proserpina: {e}");
            return ExitCode::from(e.exit_code());
        }
    };

    let policy = RetryPolicy::NONE; // fail fast on validation
    let mut all_ok = true;
    println!("Checking {} authed provider(s):\n", configs.len());
    for cfg in &configs {
        let label = format!("{} ({})", identify_provider(cfg), cfg.model);
        match validate_provider(cfg, &policy) {
            Ok(()) => println!("  ✓ {label}"),
            Err(e) => {
                println!("  ✗ {label}");
                println!("    {e}");
                println!("    This key is set but invalid. Check or remove it.");
                all_ok = false;
            }
        }
    }
    if all_ok {
        println!("\nAll keys valid.");
        ExitCode::SUCCESS
    } else {
        println!("\nSome keys failed. Run `proserpina auth list` to see which are set.");
        ExitCode::from(11)
    }
}

#[cfg(not(feature = "backend-http"))]
fn cmd_auth_check(_config: Option<&std::path::Path>) -> ExitCode {
    eprintln!("Built without `backend-http`; no HTTP providers to check.");
    ExitCode::FAILURE
}

#[cfg(feature = "backend-http")]
fn identify_provider(cfg: &proserpina::backend::http::HttpConfig) -> String {
    // Match the model/base_url against the registry to identify the provider name.
    for p in proserpina::backend::roster::Provider::registry() {
        if p.model() == cfg.model || p.base_url() == cfg.base_url {
            return p.name().to_owned();
        }
    }
    "custom".to_owned()
}

#[cfg(feature = "backend-http")]
fn cmd_auth_list(config: Option<&std::path::Path>) -> ExitCode {
    use proserpina::backend::credentials::Credentials;
    use proserpina::backend::roster::Provider;

    let creds = Credentials::discover_or(config).unwrap_or_default();
    println!("Provider key status:\n");
    for p in Provider::registry() {
        let env_set = std::env::var(p.key_env_var()).is_ok();
        let config_set = creds
            .override_for(p.name())
            .and_then(|o| o.api_key.as_ref())
            .is_some();
        let status = if env_set || config_set {
            "✓ set"
        } else {
            "  —"
        };
        let source = if env_set {
            "(env)"
        } else if config_set {
            "(config)"
        } else {
            ""
        };
        println!("  {status}  {:12} {} {source}", p.name(), p.key_env_var());
    }
    println!("\nUse `proserpina auth check` to validate that keys actually work.");
    ExitCode::SUCCESS
}

#[cfg(not(feature = "backend-http"))]
fn cmd_auth_list(_config: Option<&std::path::Path>) -> ExitCode {
    eprintln!("Built without `backend-http`.");
    ExitCode::FAILURE
}

fn cmd_panels(config: Option<&std::path::Path>) -> ExitCode {
    println!("Built-in panels:\n");
    println!("  default  1 critic   Devil's Advocate");
    println!("  duo      2 critics  Devil's Advocate + Methodologist");
    println!("  panel    5 critics  Devil's Advocate, Methodologist, Red Team,");
    println!("                      Domain Expert, Editor");

    #[cfg(feature = "backend-http")]
    {
        if let Ok(creds) = proserpina::backend::credentials::Credentials::discover_or(config) {
            if !creds.panels().is_empty() {
                println!("\nCustom panels (from config):\n");
                for (name, panel) in creds.panels() {
                    let count = panel.personas.len();
                    let names: Vec<&str> = panel.personas.iter().map(|p| p.name.as_str()).collect();
                    println!("  {name:<10} {count} critics  {}", names.join(", "));
                }
            }
        }
    }
    println!("\nUse `--panel <name>` with `proserpina critique`.");
    ExitCode::SUCCESS
}

#[cfg(all(feature = "cli", feature = "backend-http"))]
fn cmd_auth_login(provider: Option<&str>, model_override: Option<&str>) -> ExitCode {
    use proserpina::auth::api_key::run_api_key_flow;
    use proserpina::auth::tui_ratatui::RatatuiAuthUi;
    use proserpina::auth::{auth_registry, find_provider_auth, AuthStore, AuthUi};

    let ui = RatatuiAuthUi;

    // Determine which provider to authenticate.
    let provider_auth = if let Some(name) = provider {
        match find_provider_auth(name) {
            Some(p) => p,
            None => {
                ui.show_error(&format!("Unknown provider: {name}"));
                ui.show_error(&format!(
                    "Available: {}",
                    auth_registry()
                        .iter()
                        .map(|p| p.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                return ExitCode::FAILURE;
            }
        }
    } else {
        // Show provider selector.
        let registry = auth_registry();
        match ui.select_provider(registry) {
            Some(idx) => &registry[idx],
            None => {
                ui.show_status("Cancelled.");
                return ExitCode::SUCCESS;
            }
        }
    };

    // Run the appropriate auth flow.
    let key = match &provider_auth.method {
        proserpina::auth::AuthMethod::ApiKey { .. } => match run_api_key_flow(&ui, provider_auth) {
            Ok(k) => k,
            Err(e) => {
                ui.show_error(&format!("Auth failed: {e}"));
                return ExitCode::FAILURE;
            }
        },
        #[cfg(feature = "cli")]
        proserpina::auth::AuthMethod::OAuth {
            client_id,
            authorize_url,
            token_url,
            scope,
        } => {
            ui.show_status("Opening browser for authentication...");
            match proserpina::auth::oauth::run_oauth_flow(
                client_id,
                authorize_url,
                token_url,
                scope,
            ) {
                Ok(tokens) => {
                    let mut store = match AuthStore::discover() {
                        Ok(s) => s,
                        Err(e) => {
                            ui.show_error(&format!("Failed to open credential store: {e}"));
                            return ExitCode::FAILURE;
                        }
                    };
                    if let Err(e) = store.store_oauth(
                        provider_auth.name,
                        &tokens.access,
                        &tokens.refresh,
                        tokens.expires_ms,
                    ) {
                        ui.show_error(&format!("Failed to store tokens: {e}"));
                        return ExitCode::FAILURE;
                    }
                    // Store model override if specified.
                    if let Some(model) = model_override {
                        if let Err(e) = store.store_model(provider_auth.name, model) {
                            ui.show_error(&format!("Failed to store model override: {e}"));
                            return ExitCode::FAILURE;
                        }
                    }
                    ui.show_success(&format!("{} authenticated via OAuth.", provider_auth.name));
                    return ExitCode::SUCCESS;
                }
                Err(e) => {
                    ui.show_error(&format!("OAuth failed: {e}"));
                    return ExitCode::FAILURE;
                }
            }
        }
    };

    // Store the credential.
    let mut store = match AuthStore::discover() {
        Ok(s) => s,
        Err(e) => {
            ui.show_error(&format!("Failed to open credential store: {e}"));
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = store.store_api_key(provider_auth.name, &key) {
        ui.show_error(&format!("Failed to store credential: {e}"));
        return ExitCode::FAILURE;
    }

    // Store model override if specified.
    if let Some(model) = model_override {
        if let Err(e) = store.store_model(provider_auth.name, model) {
            ui.show_error(&format!("Failed to store model override: {e}"));
            return ExitCode::FAILURE;
        }
    }

    ui.show_success(&format!(
        "{} authenticated. Key stored.",
        provider_auth.name
    ));
    ExitCode::SUCCESS
}

#[cfg(not(all(feature = "cli", feature = "backend-http")))]
fn cmd_auth_login(_provider: Option<&str>, _model: Option<&str>) -> ExitCode {
    eprintln!("Built without `backend-http`; no auth available.");
    ExitCode::FAILURE
}

#[cfg(all(feature = "cli", feature = "backend-http"))]
fn cmd_auth_logout(provider: &str) -> ExitCode {
    use proserpina::auth::AuthStore;
    let mut store = match AuthStore::discover() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open credential store: {e}");
            return ExitCode::FAILURE;
        }
    };
    match store.remove(provider) {
        Ok(()) => {
            println!("✓ Removed credentials for {provider}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("✗ Failed: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(all(feature = "cli", feature = "backend-http")))]
fn cmd_auth_logout(_provider: &str) -> ExitCode {
    eprintln!("Built without `backend-http`.");
    ExitCode::FAILURE
}

/// Emits an error appropriately: structured JSON on stderr if `--json`, else
/// human-readable prose with actionable guidance.
fn emit_error(err: &proserpina::ProserpinaError, json: bool) {
    if json {
        #[cfg(feature = "json")]
        {
            eprintln!("{}", err.to_error_json());
            return;
        }
    }
    // Human-readable: the error message + actionable guidance for common cases.
    eprintln!("proserpina: {err}");
    let msg = err.to_string();
    if msg.contains("401") || msg.contains("invalid_api_key") || msg.contains("Incorrect API key") {
        eprintln!();
        eprintln!("  A provider rejected your API key. Run `proserpina auth check` to");
        eprintln!("  diagnose which key is invalid.");
    } else if msg.contains("429") {
        eprintln!();
        eprintln!("  A provider rate-limited the request. Consider increasing");
        eprintln!("  --max-attempts or --timeout, or reducing the panel size.");
    }
}

// CLI flag sprawl is inherent to a subcommand entry point.
#[allow(clippy::too_many_arguments)]
fn run_critique_cmd(
    input: &std::path::Path,
    echo: bool,
    seed: Option<u64>,
    config: Option<&std::path::Path>,
    json: bool,
    dry_run: bool,
    panel: Option<&str>,
    max_attempts: Option<u32>,
    timeout: Option<u64>,
    language: Option<&str>,
    exclude: Option<&str>,
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

        // Refresh any expired OAuth tokens before the run.
        if let Ok(mut auth_store) = proserpina::auth::AuthStore::discover() {
            let _ = auth_store.refresh_expired_oauth();
        }

        let retry_config = proserpina::backend::credentials::Credentials::discover_or(config)
            .map(|c| c.retry().clone())
            .unwrap_or_default();
        let policy =
            proserpina::backend::http::RetryPolicy::resolve(&retry_config, max_attempts, timeout);
        // Parse comma-separated --exclude into a Vec<String>.
        let excludes: Vec<String> = exclude
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        if dry_run {
            return proserpina::cli::plan_critique(
                &text, &source, seed, config, json, panel, &excludes,
            );
        }

        // Progress output for humans (stderr, so stdout stays clean for piping).
        if !json {
            let panel_name = panel.unwrap_or("default");
            eprintln!("Proserpina v0.2.1 — panel: {panel_name}\n");
        }

        proserpina::cli::run_critique(
            &text, &source, seed, config, json, panel, policy, language, &excludes,
        )
    }

    #[cfg(not(feature = "backend-http"))]
    {
        let _ = (
            seed,
            config,
            json,
            dry_run,
            panel,
            max_attempts,
            timeout,
            language,
            exclude,
        );
        let mut report = proserpina::cli::run_critique_echo(&text, &source)?;
        report.push_str("\n_(built without `backend-http`; used the echo backend)_\n");
        Ok(report)
    }
}
