//! Tramp - CLI tool for proxying commands with pre/post hooks and trampolines.
//!
//! This library provides the core functionality for tramp, including:
//! - Configuration file parsing and cascade discovery
//! - Rule matching and command rewriting
//! - Hook execution with environment variable context
//! - Command execution with proper stdio handling
//!
//! # Example
//!
//! ```no_run
//! use tramp_cli::config::load_merged_config;
//! use tramp_cli::rules::{compile_rules, find_matching_rule, MatchContext};
//! use std::path::Path;
//!
//! let cwd = std::env::current_dir().unwrap();
//! let config = load_merged_config(&cwd).unwrap();
//! let rules = compile_rules(&config).unwrap();
//!
//! let ctx = MatchContext {
//!     binary_path: Path::new("/usr/local/bin/cargo"),
//!     cwd: &cwd,
//!     args: &["build".to_string()],
//! };
//!
//! if let Some(rule) = find_matching_rule(&rules, &ctx) {
//!     println!("Matched rule from: {:?}", rule.source);
//! }
//! ```

pub mod config;
pub mod error;
pub mod exec;
pub mod hooks;
pub mod rules;

pub use error::{Result, TrampError};
