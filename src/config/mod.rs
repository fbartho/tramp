//! Configuration loading and parsing for tramp.
//!
//! This module handles:
//! - TOML config file parsing
//! - Directory cascade discovery
//! - Config merging

pub mod cascade;
pub mod parser;
pub mod types;

pub use cascade::{discover_configs, load_merged_config, merge_configs, user_config_path};
pub use parser::{parse_config_file, parse_config_str};
pub use types::{Config, LoadedConfig, MergedConfig, Rule, RuleWithSource};
