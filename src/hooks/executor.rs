use crate::error::{Result, TrampError};
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

/// Type of hook being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookType {
	Pre,
	Post,
	Intercept,
}

impl HookType {
	/// Get the string representation for TRAMP_HOOK_TYPE env var.
	pub fn as_str(&self) -> &'static str {
		match self {
			HookType::Pre => "pre",
			HookType::Post => "post",
			HookType::Intercept => "intercept",
		}
	}
}

/// Context for hook execution.
#[derive(Debug)]
pub struct HookContext<'a> {
	/// Original binary path.
	pub original_binary: &'a Path,

	/// Original arguments.
	pub original_args: &'a [String],

	/// Current working directory.
	pub cwd: &'a Path,

	/// Type of hook.
	pub hook_type: HookType,

	/// Executed binary (for post-hooks, after any rewrites).
	pub executed_binary: Option<&'a Path>,

	/// Executed arguments (for post-hooks, after any rewrites).
	pub executed_args: Option<&'a [String]>,

	/// Exit code from the command (for post-hooks).
	pub exit_code: Option<i32>,
}

/// Build environment variables for hook execution.
pub fn build_hook_env(ctx: &HookContext) -> HashMap<String, String> {
	let mut env = HashMap::new();

	// Always set
	env.insert(
		"TRAMP_ORIGINAL_BINARY".to_string(),
		ctx.original_binary.to_string_lossy().to_string(),
	);
	env.insert(
		"TRAMP_ORIGINAL_ARGS".to_string(),
		ctx.original_args.join(" "),
	);
	env.insert(
		"TRAMP_ORIGINAL_ARGC".to_string(),
		ctx.original_args.len().to_string(),
	);

	// Individual args
	for (i, arg) in ctx.original_args.iter().enumerate() {
		env.insert(format!("TRAMP_ORIGINAL_ARG_{}", i), arg.clone());
	}

	env.insert(
		"TRAMP_CWD".to_string(),
		ctx.cwd.to_string_lossy().to_string(),
	);
	env.insert(
		"TRAMP_HOOK_TYPE".to_string(),
		ctx.hook_type.as_str().to_string(),
	);

	// Post-hook only
	if let Some(executed_binary) = ctx.executed_binary {
		env.insert(
			"TRAMP_EXECUTED_BINARY".to_string(),
			executed_binary.to_string_lossy().to_string(),
		);
	}

	if let Some(executed_args) = ctx.executed_args {
		env.insert("TRAMP_EXECUTED_ARGS".to_string(), executed_args.join(" "));
	}

	if let Some(exit_code) = ctx.exit_code {
		env.insert("TRAMP_EXIT_CODE".to_string(), exit_code.to_string());
	}

	env
}

/// Execute a hook script.
pub fn execute_hook(hook_path: &Path, ctx: &HookContext) -> Result<i32> {
	let env = build_hook_env(ctx);

	let mut cmd = Command::new("sh");
	cmd.arg("-c")
		.arg(hook_path.to_string_lossy().to_string())
		.current_dir(ctx.cwd)
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.envs(&env);

	let status = cmd.status().map_err(|source| TrampError::HookFailed {
		hook_path: hook_path.to_path_buf(),
		source,
	})?;

	Ok(status.code().unwrap_or(-1))
}

/// Execute a pre-hook. Returns error if hook fails (non-zero exit).
pub fn execute_pre_hook(hook_path: &Path, ctx: &HookContext) -> Result<()> {
	let exit_code = execute_hook(hook_path, ctx)?;

	if exit_code != 0 {
		return Err(TrampError::HookNonZeroExit {
			hook_path: hook_path.to_path_buf(),
			exit_code,
		});
	}

	Ok(())
}

/// Execute a post-hook. Returns the hook's exit code (doesn't fail on non-zero).
pub fn execute_post_hook(hook_path: &Path, ctx: &HookContext) -> Result<i32> {
	execute_hook(hook_path, ctx)
}

/// Execute an intercept hook. The hook replaces command execution entirely.
/// Returns the hook's exit code.
pub fn execute_intercept_hook(hook_path: &Path, ctx: &HookContext) -> Result<i32> {
	execute_hook(hook_path, ctx)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hook_type_as_str() {
		assert_eq!(HookType::Pre.as_str(), "pre");
		assert_eq!(HookType::Post.as_str(), "post");
		assert_eq!(HookType::Intercept.as_str(), "intercept");
	}

	#[test]
	fn test_build_hook_env_basic() {
		let ctx = HookContext {
			original_binary: Path::new("/usr/local/bin/cargo"),
			original_args: &["build".to_string(), "--release".to_string()],
			cwd: Path::new("/home/user/project"),
			hook_type: HookType::Pre,
			executed_binary: None,
			executed_args: None,
			exit_code: None,
		};

		let env = build_hook_env(&ctx);

		assert_eq!(
			env.get("TRAMP_ORIGINAL_BINARY").unwrap(),
			"/usr/local/bin/cargo"
		);
		assert_eq!(env.get("TRAMP_ORIGINAL_ARGS").unwrap(), "build --release");
		assert_eq!(env.get("TRAMP_ORIGINAL_ARGC").unwrap(), "2");
		assert_eq!(env.get("TRAMP_ORIGINAL_ARG_0").unwrap(), "build");
		assert_eq!(env.get("TRAMP_ORIGINAL_ARG_1").unwrap(), "--release");
		assert_eq!(env.get("TRAMP_CWD").unwrap(), "/home/user/project");
		assert_eq!(env.get("TRAMP_HOOK_TYPE").unwrap(), "pre");

		// Post-hook only vars should not be set
		assert!(env.get("TRAMP_EXECUTED_BINARY").is_none());
		assert!(env.get("TRAMP_EXECUTED_ARGS").is_none());
		assert!(env.get("TRAMP_EXIT_CODE").is_none());
	}

	#[test]
	fn test_build_hook_env_post_hook() {
		let executed_args = vec![
			"build".to_string(),
			"--release".to_string(),
			"--locked".to_string(),
		];
		let ctx = HookContext {
			original_binary: Path::new("/usr/local/bin/cargo"),
			original_args: &["build".to_string(), "--release".to_string()],
			cwd: Path::new("/home/user/project"),
			hook_type: HookType::Post,
			executed_binary: Some(Path::new("/usr/local/bin/cargo")),
			executed_args: Some(&executed_args),
			exit_code: Some(0),
		};

		let env = build_hook_env(&ctx);

		assert_eq!(env.get("TRAMP_HOOK_TYPE").unwrap(), "post");
		assert_eq!(
			env.get("TRAMP_EXECUTED_BINARY").unwrap(),
			"/usr/local/bin/cargo"
		);
		assert_eq!(
			env.get("TRAMP_EXECUTED_ARGS").unwrap(),
			"build --release --locked"
		);
		assert_eq!(env.get("TRAMP_EXIT_CODE").unwrap(), "0");
	}

	#[test]
	fn test_build_hook_env_empty_args() {
		let ctx = HookContext {
			original_binary: Path::new("/usr/local/bin/cargo"),
			original_args: &[],
			cwd: Path::new("/home/user/project"),
			hook_type: HookType::Pre,
			executed_binary: None,
			executed_args: None,
			exit_code: None,
		};

		let env = build_hook_env(&ctx);

		assert_eq!(env.get("TRAMP_ORIGINAL_ARGS").unwrap(), "");
		assert_eq!(env.get("TRAMP_ORIGINAL_ARGC").unwrap(), "0");
		assert!(env.get("TRAMP_ORIGINAL_ARG_0").is_none());
	}

	#[test]
	fn test_build_hook_env_intercept_hook() {
		// Intercept hooks receive the same context as pre-hooks,
		// plus executed_binary/executed_args (what would have run)
		let executed_args = vec!["deploy".to_string(), "--env=staging".to_string()];
		let ctx = HookContext {
			original_binary: Path::new("/usr/local/bin/deploy"),
			original_args: &["deploy".to_string(), "--env=staging".to_string()],
			cwd: Path::new("/home/user/my-app"),
			hook_type: HookType::Intercept,
			executed_binary: Some(Path::new("/usr/local/bin/deploy")),
			executed_args: Some(&executed_args),
			exit_code: None, // No exit code yet - command hasn't run
		};

		let env = build_hook_env(&ctx);

		// Verify intercept hook type
		assert_eq!(env.get("TRAMP_HOOK_TYPE").unwrap(), "intercept");

		// Verify original command info
		assert_eq!(
			env.get("TRAMP_ORIGINAL_BINARY").unwrap(),
			"/usr/local/bin/deploy"
		);
		assert_eq!(
			env.get("TRAMP_ORIGINAL_ARGS").unwrap(),
			"deploy --env=staging"
		);
		assert_eq!(env.get("TRAMP_ORIGINAL_ARGC").unwrap(), "2");
		assert_eq!(env.get("TRAMP_ORIGINAL_ARG_0").unwrap(), "deploy");
		assert_eq!(env.get("TRAMP_ORIGINAL_ARG_1").unwrap(), "--env=staging");
		assert_eq!(env.get("TRAMP_CWD").unwrap(), "/home/user/my-app");

		// Verify executed command info is available (for logging/debugging)
		assert_eq!(
			env.get("TRAMP_EXECUTED_BINARY").unwrap(),
			"/usr/local/bin/deploy"
		);
		assert_eq!(
			env.get("TRAMP_EXECUTED_ARGS").unwrap(),
			"deploy --env=staging"
		);

		// No exit code for intercept hooks (command doesn't run)
		assert!(env.get("TRAMP_EXIT_CODE").is_none());
	}
}
