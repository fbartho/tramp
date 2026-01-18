use serde::Deserialize;
use std::path::PathBuf;

/// Top-level configuration from a `.tramp.toml` file.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
	/// If true, stop directory cascade and jump directly to ~/.tramp.toml.
	#[serde(default)]
	pub root: bool,

	/// If true, don't allow local developer hooks to override this config.
	#[serde(default)]
	pub no_external_lookup: bool,

	/// Environment variable name that, if truthy, skips ~/.tramp.toml lookup.
	/// Useful for CI environments.
	#[serde(default)]
	pub root_config_lookup_disable_env_var: Option<String>,

	/// Rules for matching and transforming commands.
	/// First matching rule wins.
	#[serde(default)]
	pub rules: Vec<Rule>,
}

/// A rule for matching and transforming commands.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Rule {
	/// Regex pattern to match the command binary path.
	pub binary_pattern: Option<String>,

	/// Regex pattern to match the current working directory.
	pub cwd_pattern: Option<String>,

	/// Regex substitution for arguments (mutually exclusive with command_rewrite and alternate_command).
	/// Format: "s/pattern/replacement/" or "s/pattern/replacement/g" for global.
	pub arg_rewrite: Option<String>,

	/// Regex substitution for the entire command string (mutually exclusive with arg_rewrite and alternate_command).
	/// Format: "s/pattern/replacement/" or "s/pattern/replacement/g" for global.
	pub command_rewrite: Option<String>,

	/// Replacement command to execute instead (mutually exclusive with arg_rewrite and command_rewrite).
	pub alternate_command: Option<String>,

	/// Path to pre-hook script. Runs before the command.
	pub pre_hook: Option<PathBuf>,

	/// Path to post-hook script. Runs after the command.
	pub post_hook: Option<PathBuf>,

	/// Path to intercept hook script. Replaces command execution entirely.
	pub intercept_hook: Option<PathBuf>,
}

/// A loaded configuration with its source path for debugging/display.
#[derive(Debug, Clone)]
pub struct LoadedConfig {
	/// The parsed configuration.
	pub config: Config,

	/// The path this config was loaded from.
	pub path: PathBuf,
}

/// Merged configuration from multiple config files in the cascade.
#[derive(Debug, Clone, Default)]
pub struct MergedConfig {
	/// All rules from all configs, in cascade order (first match wins).
	pub rules: Vec<RuleWithSource>,

	/// Whether external lookup is disabled (from any config in cascade).
	pub no_external_lookup: bool,
}

/// A rule with its source config path for debugging/display.
#[derive(Debug, Clone)]
pub struct RuleWithSource {
	/// The rule itself.
	pub rule: Rule,

	/// The config file this rule came from.
	pub source: PathBuf,
}

impl Rule {
	/// Validate that mutually exclusive fields are not both set.
	pub fn validate(&self) -> Result<(), crate::error::TrampError> {
		let rewrite_fields = [
			("arg_rewrite", self.arg_rewrite.is_some()),
			("command_rewrite", self.command_rewrite.is_some()),
			("alternate_command", self.alternate_command.is_some()),
		];

		let set_fields: Vec<_> = rewrite_fields
			.iter()
			.filter(|(_, is_set)| *is_set)
			.map(|(name, _)| *name)
			.collect();

		if set_fields.len() > 1 {
			return Err(crate::error::TrampError::MutuallyExclusive {
				option1: set_fields[0].to_string(),
				option2: set_fields[1].to_string(),
			});
		}

		Ok(())
	}
}

impl Config {
	/// Validate all rules in this config.
	pub fn validate(&self) -> Result<(), crate::error::TrampError> {
		for rule in &self.rules {
			rule.validate()?;
		}
		Ok(())
	}
}
