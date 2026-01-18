use crate::config::parser::parse_config_file;
use crate::config::types::{LoadedConfig, MergedConfig, RuleWithSource};
use crate::error::{Result, TrampError};
use std::path::{Path, PathBuf};

/// Discover and load all config files in the cascade.
///
/// The cascade order is:
/// 1. Start from `start_dir` and look for `.tramp.toml`
/// 2. If found and `root = true`, skip to user config only
/// 3. Otherwise, continue up the directory tree
/// 4. Finally, check ~/.tramp.toml (unless disabled)
///
/// Returns configs in cascade order (most specific first).
pub fn discover_configs(start_dir: &Path) -> Result<Vec<LoadedConfig>> {
	let mut configs = Vec::new();
	let mut current_dir = start_dir.to_path_buf();
	let mut skip_cascade = false;

	// Walk up the directory tree
	loop {
		let config_path = current_dir.join(".tramp.toml");

		if config_path.exists() {
			let config = parse_config_file(&config_path)?;

			// Check if external lookup is disabled
			if config.no_external_lookup {
				// Only use this config, skip everything else
				configs.push(LoadedConfig {
					config,
					path: config_path,
				});
				return Ok(configs);
			}

			// Check if we should skip cascade and jump to user config
			if config.root {
				skip_cascade = true;
			}

			configs.push(LoadedConfig {
				config,
				path: config_path,
			});

			if skip_cascade {
				break;
			}
		}

		// Move to parent directory
		if let Some(parent) = current_dir.parent() {
			current_dir = parent.to_path_buf();
		} else {
			break;
		}
	}

	// Check user config unless disabled by env var
	if let Some(user_config) = load_user_config(&configs)? {
		configs.push(user_config);
	}

	Ok(configs)
}

/// Load the user's ~/.tramp.toml if it exists and isn't disabled.
fn load_user_config(existing_configs: &[LoadedConfig]) -> Result<Option<LoadedConfig>> {
	// Check if any config disables user config lookup via env var
	for loaded in existing_configs {
		if let Some(ref env_var) = loaded.config.root_config_lookup_disable_env_var
			&& is_env_truthy(env_var)
		{
			return Ok(None);
		}
	}

	let home_dir = dirs::home_dir().ok_or(TrampError::HomeDirectoryNotFound)?;
	let user_config_path = home_dir.join(".tramp.toml");

	if user_config_path.exists() {
		let config = parse_config_file(&user_config_path)?;
		Ok(Some(LoadedConfig {
			config,
			path: user_config_path,
		}))
	} else {
		Ok(None)
	}
}

/// Check if an environment variable is set to a truthy value.
fn is_env_truthy(var_name: &str) -> bool {
	match std::env::var(var_name) {
		Ok(value) => {
			let lower = value.to_lowercase();
			!value.is_empty() && lower != "0" && lower != "false" && lower != "no"
		}
		Err(_) => false,
	}
}

/// Merge multiple configs into a single effective config.
///
/// Rules are collected in cascade order (first match wins).
/// The `no_external_lookup` flag is set if any config has it.
pub fn merge_configs(configs: &[LoadedConfig]) -> MergedConfig {
	let mut merged = MergedConfig::default();

	for loaded in configs {
		// Collect rules with their source
		for rule in &loaded.config.rules {
			merged.rules.push(RuleWithSource {
				rule: rule.clone(),
				source: loaded.path.clone(),
			});
		}

		// Track if any config disables external lookup
		if loaded.config.no_external_lookup {
			merged.no_external_lookup = true;
		}
	}

	merged
}

/// Convenience function to discover, load, and merge configs from a directory.
pub fn load_merged_config(start_dir: &Path) -> Result<MergedConfig> {
	let configs = discover_configs(start_dir)?;
	Ok(merge_configs(&configs))
}

/// Get the path to the user's config file.
pub fn user_config_path() -> Result<PathBuf> {
	let home_dir = dirs::home_dir().ok_or(TrampError::HomeDirectoryNotFound)?;
	Ok(home_dir.join(".tramp.toml"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_is_env_truthy() {
		// SAFETY: These env var operations are safe in single-threaded test context
		unsafe {
			// Not set
			std::env::remove_var("TEST_TRAMP_ENV_1");
			assert!(!is_env_truthy("TEST_TRAMP_ENV_1"));

			// Empty string
			std::env::set_var("TEST_TRAMP_ENV_2", "");
			assert!(!is_env_truthy("TEST_TRAMP_ENV_2"));

			// "0"
			std::env::set_var("TEST_TRAMP_ENV_3", "0");
			assert!(!is_env_truthy("TEST_TRAMP_ENV_3"));

			// "false"
			std::env::set_var("TEST_TRAMP_ENV_4", "false");
			assert!(!is_env_truthy("TEST_TRAMP_ENV_4"));

			// "FALSE"
			std::env::set_var("TEST_TRAMP_ENV_5", "FALSE");
			assert!(!is_env_truthy("TEST_TRAMP_ENV_5"));

			// "no"
			std::env::set_var("TEST_TRAMP_ENV_6", "no");
			assert!(!is_env_truthy("TEST_TRAMP_ENV_6"));

			// "1" - truthy
			std::env::set_var("TEST_TRAMP_ENV_7", "1");
			assert!(is_env_truthy("TEST_TRAMP_ENV_7"));

			// "true" - truthy
			std::env::set_var("TEST_TRAMP_ENV_8", "true");
			assert!(is_env_truthy("TEST_TRAMP_ENV_8"));

			// Any other value - truthy
			std::env::set_var("TEST_TRAMP_ENV_9", "yes");
			assert!(is_env_truthy("TEST_TRAMP_ENV_9"));

			// Cleanup
			for i in 1..=9 {
				std::env::remove_var(format!("TEST_TRAMP_ENV_{}", i));
			}
		}
	}

	#[test]
	fn test_user_config_path() {
		let path = user_config_path();
		assert!(path.is_ok());
		let path = path.unwrap();
		assert!(path.ends_with(".tramp.toml"));
	}
}
