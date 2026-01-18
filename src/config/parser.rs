use crate::config::types::Config;
use crate::error::{Result, TrampError};
use std::path::Path;

/// Parse a config file from the given path.
pub fn parse_config_file(path: &Path) -> Result<Config> {
	let content = std::fs::read_to_string(path).map_err(|source| TrampError::ConfigReadError {
		path: path.to_path_buf(),
		source,
	})?;

	parse_config_str(&content, path)
}

/// Parse a config from a string (useful for testing).
pub fn parse_config_str(content: &str, path: &Path) -> Result<Config> {
	let config: Config =
		toml::from_str(content).map_err(|source| TrampError::ConfigParseError {
			path: path.to_path_buf(),
			source,
		})?;

	// Validate the parsed config
	config.validate()?;

	Ok(config)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	#[test]
	fn test_parse_empty_config() {
		let content = "";
		let path = PathBuf::from("test.toml");
		let config = parse_config_str(content, &path).unwrap();

		assert!(!config.root);
		assert!(!config.no_external_lookup);
		assert!(config.root_config_lookup_disable_env_var.is_none());
		assert!(config.rules.is_empty());
	}

	#[test]
	fn test_parse_basic_config() {
		let content = r#"
root = true
no-external-lookup = true
root-config-lookup-disable-env-var = "CI"
"#;
		let path = PathBuf::from("test.toml");
		let config = parse_config_str(content, &path).unwrap();

		assert!(config.root);
		assert!(config.no_external_lookup);
		assert_eq!(
			config.root_config_lookup_disable_env_var,
			Some("CI".to_string())
		);
	}

	#[test]
	fn test_parse_rules_array_of_tables() {
		let content = r#"
[[rules]]
binary_pattern = ".*/cargo$"
arg_rewrite = "s/^build$/build --release/"
pre_hook = "/path/to/hook.sh"

[[rules]]
binary_pattern = ".*/npm$"
alternate_command = "/usr/local/bin/pnpm"
"#;
		let path = PathBuf::from("test.toml");
		let config = parse_config_str(content, &path).unwrap();

		assert_eq!(config.rules.len(), 2);

		let rule1 = &config.rules[0];
		assert_eq!(rule1.binary_pattern, Some(".*/cargo$".to_string()));
		assert_eq!(
			rule1.arg_rewrite,
			Some("s/^build$/build --release/".to_string())
		);
		assert_eq!(rule1.pre_hook, Some(PathBuf::from("/path/to/hook.sh")));

		let rule2 = &config.rules[1];
		assert_eq!(rule2.binary_pattern, Some(".*/npm$".to_string()));
		assert_eq!(
			rule2.alternate_command,
			Some("/usr/local/bin/pnpm".to_string())
		);
	}

	#[test]
	fn test_parse_rules_inline_tables() {
		let content = r#"
rules = [
    { binary_pattern = ".*/cargo$", arg_rewrite = "s/^build$/build --release/" },
    { binary_pattern = ".*/npm$", alternate_command = "/usr/local/bin/pnpm" },
]
"#;
		let path = PathBuf::from("test.toml");
		let config = parse_config_str(content, &path).unwrap();

		assert_eq!(config.rules.len(), 2);
	}

	#[test]
	fn test_mutually_exclusive_rewrite_options() {
		let content = r#"
[[rules]]
binary_pattern = ".*/cargo$"
arg_rewrite = "s/foo/bar/"
command_rewrite = "s/baz/qux/"
"#;
		let path = PathBuf::from("test.toml");
		let result = parse_config_str(content, &path);

		assert!(result.is_err());
		match result.unwrap_err() {
			TrampError::MutuallyExclusive { option1, option2 } => {
				assert_eq!(option1, "arg_rewrite");
				assert_eq!(option2, "command_rewrite");
			}
			_ => panic!("Expected MutuallyExclusive error"),
		}
	}

	#[test]
	fn test_parse_intercept_hook() {
		let content = r#"
[[rules]]
binary_pattern = ".*/deploy$"
intercept_hook = "/path/to/intercept.sh"

[[rules]]
binary_pattern = ".*/my-tool$"
pre_hook = "/path/to/pre.sh"
intercept_hook = "/path/to/intercept.sh"
post_hook = "/path/to/post.sh"
"#;
		let path = PathBuf::from("test.toml");
		let config = parse_config_str(content, &path).unwrap();

		assert_eq!(config.rules.len(), 2);

		// Rule with only intercept_hook
		let rule1 = &config.rules[0];
		assert_eq!(rule1.binary_pattern, Some(".*/deploy$".to_string()));
		assert_eq!(
			rule1.intercept_hook,
			Some(PathBuf::from("/path/to/intercept.sh"))
		);
		assert!(rule1.pre_hook.is_none());
		assert!(rule1.post_hook.is_none());

		// Rule with all hook types (intercept + pre + post is valid)
		let rule2 = &config.rules[1];
		assert_eq!(rule2.binary_pattern, Some(".*/my-tool$".to_string()));
		assert_eq!(rule2.pre_hook, Some(PathBuf::from("/path/to/pre.sh")));
		assert_eq!(
			rule2.intercept_hook,
			Some(PathBuf::from("/path/to/intercept.sh"))
		);
		assert_eq!(rule2.post_hook, Some(PathBuf::from("/path/to/post.sh")));
	}
}
