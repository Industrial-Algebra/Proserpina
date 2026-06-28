// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the CLI's critique entry point.

#![cfg(feature = "cli")]

use proserpina::cli::run_critique_echo;

#[test]
fn run_critique_produces_a_markdown_report() {
    let input = "# Roadmap\n\nWe will ship the thing.";
    let markdown =
        run_critique_echo(input, "roadmap.md").expect("echo-backed critique never fails");

    assert!(markdown.starts_with("# Critique Report"));
    // The default panel has at least one critic, so there should be findings.
    assert!(!markdown.contains("No findings."));
}

#[test]
fn run_critique_marks_the_source_in_the_report() {
    // The report should record which document was critiqued.
    let markdown =
        run_critique_echo("some text", "plan.md").expect("echo-backed critique never fails");
    assert!(markdown.contains("plan.md"));
}

#[test]
fn run_critique_works_with_anonymous_subject() {
    let markdown =
        run_critique_echo("anonymous doc", "").expect("echo-backed critique never fails");
    assert!(markdown.starts_with("# Critique Report"));
}
