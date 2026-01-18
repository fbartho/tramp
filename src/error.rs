use std::path::PathBuf;

/// Library-level structured errors for tramp.
///
/// Use `thiserror` for structured errors that library consumers can match on.
/// The CLI binary wraps these with `anyhow` for rich context chains.
#[derive(Debug, thiserror::Error)]
pub enum TrampError {
	#[error("Config file not found: {path}")]
	ConfigNotFound { path: PathBuf },

	#[error("Failed to read config file: {path}")]
	ConfigReadError {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("Failed to parse config file: {path}")]
	ConfigParseError {
		path: PathBuf,
		#[source]
		source: toml::de::Error,
	},

	#[error("Invalid regex pattern in rule: {pattern}")]
	InvalidRegex {
		pattern: String,
		#[source]
		source: regex::Error,
	},

	#[error("Mutually exclusive options: {option1} and {option2}")]
	MutuallyExclusive { option1: String, option2: String },

	#[error("Hook execution failed: {hook_path}")]
	HookFailed {
		hook_path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("Hook returned non-zero exit code: {hook_path} (exit code: {exit_code})")]
	HookNonZeroExit { hook_path: PathBuf, exit_code: i32 },

	#[error("Command execution failed: {command}")]
	CommandFailed {
		command: String,
		#[source]
		source: std::io::Error,
	},

	#[error("Command not found: {command}")]
	CommandNotFound { command: String },

	#[error("Failed to resolve home directory")]
	HomeDirectoryNotFound,
}

/// Result type alias using TrampError.
pub type Result<T> = std::result::Result<T, TrampError>;
