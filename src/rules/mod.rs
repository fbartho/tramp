//! Rule matching and rewriting for tramp.
//!
//! This module handles:
//! - Pattern matching for binary paths and working directories
//! - Argument and command rewriting using sed-like substitutions

pub mod matcher;
pub mod rewriter;

pub use matcher::{CompiledRule, MatchContext, compile_rules, find_matching_rule};
pub use rewriter::{Substitution, rewrite_args, rewrite_command};
