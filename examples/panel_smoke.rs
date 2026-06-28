// Copyright (C) 2026 Industrial Algebra
// SPDX-License-Identifier: Apache-2.0

//! Manual smoke test: a full multi-critic multi-provider panel run.
//!
//! Run with:
//!   DEEPSEEK_API_KEY=... cargo run --features cli,backend-http --example panel_smoke -- doc.md
//!
//! Demonstrates the roster fanning 5 critics across authed providers, with the
//! summarizer clustering findings. Not a unit test — makes real, billable
//! network calls.

#![cfg(feature = "backend-http")]

use std::env;
use std::path::PathBuf;

use proserpina::backend::http::RetryPolicy;
use proserpina::cli::run_critique;

fn main() {
    let doc = env::args()
        .nth(1)
        .expect("usage: panel_smoke <path-to-doc.md>");
    let path = PathBuf::from(&doc);
    let text = std::fs::read_to_string(&path).expect("read doc");

    let report = run_critique(
        &text,
        &doc,
        3, // seed for reproducibility
        None,
        false,         // markdown (not json)
        Some("panel"), // 5-critic panel
        RetryPolicy::DEFAULT,
    )
    .expect("run should succeed");

    print!("{report}");
}
