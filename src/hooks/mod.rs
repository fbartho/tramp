//! Hook execution for tramp.
//!
//! This module handles:
//! - Pre-hook execution (before command)
//! - Post-hook execution (after command)
//! - Intercept hook execution (replaces command)
//! - Hook environment variable setup

pub mod executor;

pub use executor::{
	HookContext, HookType, build_hook_env, execute_hook, execute_intercept_hook, execute_post_hook,
	execute_pre_hook,
};
