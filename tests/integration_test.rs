#![allow(deprecated)] // assert_cmd::Command::cargo_bin is deprecated but replacement requires nightly

use predicates::prelude::*;
use std::fs;

fn tramp_cmd() -> assert_cmd::Command {
	assert_cmd::Command::cargo_bin("tramp").unwrap()
}

// ============================================================================
// CLI flag tests
// ============================================================================

#[test]
fn test_help_flag() {
	tramp_cmd()
		.arg("--help")
		.assert()
		.success()
		.stdout(predicate::str::contains("CLI tool for proxying commands"));
}

#[test]
fn test_version_flag() {
	tramp_cmd()
		.arg("--version")
		.assert()
		.success()
		.stdout(predicate::str::contains("tramp"));
}

#[test]
fn test_no_args_shows_help() {
	// With arg_required_else_help, no args should show help
	tramp_cmd()
		.assert()
		.failure()
		.stderr(predicate::str::contains("Usage"));
}

// ============================================================================
// --init tests
// ============================================================================

#[test]
fn test_init_creates_config() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	tramp_cmd()
		.arg("--init")
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("Created .tramp.toml"));

	assert!(config_path.exists());

	let content = fs::read_to_string(&config_path).unwrap();
	assert!(content.contains("root = true"));
	assert!(content.contains("[[rules]]"));
}

#[test]
fn test_init_fails_if_exists() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	// Create existing file
	fs::write(&config_path, "# existing").unwrap();

	tramp_cmd()
		.arg("--init")
		.current_dir(temp_dir.path())
		.assert()
		.failure()
		.stderr(predicate::str::contains("already exists"));
}

#[test]
fn test_init_force_overwrites() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	// Create existing file
	fs::write(&config_path, "# existing").unwrap();

	tramp_cmd()
		.args(["--init", "--force"])
		.current_dir(temp_dir.path())
		.assert()
		.success();

	let content = fs::read_to_string(&config_path).unwrap();
	assert!(content.contains("root = true"));
}

// ============================================================================
// --setup tests
// ============================================================================

#[test]
fn test_setup_generates_script() {
	tramp_cmd()
		.args(["--setup", "/usr/local/bin/cargo"])
		.assert()
		.success()
		.stdout(predicate::str::contains("#!/bin/sh"))
		.stdout(predicate::str::contains("tramp"))
		.stdout(predicate::str::contains("/usr/local/bin/cargo"));
}

// ============================================================================
// config subcommand tests
// ============================================================================

#[test]
fn test_config_validate_no_config() {
	let temp_dir = tempfile::tempdir().unwrap();

	tramp_cmd()
		.args(["config", "validate"])
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("No configuration files found"));
}

#[test]
fn test_config_validate_valid_config() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	fs::write(
		&config_path,
		r#"
root = true

[[rules]]
binary_pattern = ".*/echo$"
"#,
	)
	.unwrap();

	tramp_cmd()
		.args(["config", "validate"])
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("valid"));
}

#[test]
fn test_config_validate_invalid_config() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	fs::write(&config_path, "invalid toml [[[").unwrap();

	tramp_cmd()
		.args(["config", "validate"])
		.current_dir(temp_dir.path())
		.assert()
		.failure();
}

#[test]
fn test_config_show_displays_config() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	fs::write(
		&config_path,
		r#"
root = true

[[rules]]
binary_pattern = ".*/cargo$"
arg_rewrite = "s/build/build --release/"
"#,
	)
	.unwrap();

	tramp_cmd()
		.args(["config", "show"])
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("binary_pattern"))
		.stdout(predicate::str::contains("arg_rewrite"));
}

// ============================================================================
// Command execution tests (Unix only - these use Unix commands)
// ============================================================================

#[test]
fn test_command_not_found() {
	tramp_cmd()
		.args(["nonexistent_command_12345"])
		.assert()
		.failure()
		.stderr(predicate::str::contains("not found"));
}

#[cfg(unix)]
#[test]
fn test_run_simple_command() {
	tramp_cmd()
		.args(["echo", "hello", "world"])
		.assert()
		.success()
		.stdout(predicate::str::contains("hello world"));
}

#[cfg(unix)]
#[test]
fn test_command_exit_code_propagates() {
	tramp_cmd().args(["sh", "-c", "exit 42"]).assert().code(42);
}

#[cfg(unix)]
#[test]
fn test_command_with_args() {
	tramp_cmd()
		.args(["sh", "-c", "echo $1 $2", "--", "foo", "bar"])
		.assert()
		.success()
		.stdout(predicate::str::contains("foo bar"));
}

// ============================================================================
// Rule matching and rewriting tests (Unix only)
// ============================================================================

#[cfg(unix)]
#[test]
fn test_arg_rewrite() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	// Rewrite "hello" to "goodbye"
	fs::write(
		&config_path,
		r#"
root = true

[[rules]]
binary_pattern = ".*/echo$"
arg_rewrite = "s/hello/goodbye/"
"#,
	)
	.unwrap();

	tramp_cmd()
		.args(["echo", "hello", "world"])
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("goodbye world"));
}

#[cfg(unix)]
#[test]
fn test_alternate_command() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	// Replace "false" command with "true"
	fs::write(
		&config_path,
		r#"
root = true

[[rules]]
binary_pattern = ".*/false$"
alternate_command = "true"
"#,
	)
	.unwrap();

	// "false" normally exits with 1, but with alternate_command it uses "true" which exits 0
	tramp_cmd()
		.args(["false"])
		.current_dir(temp_dir.path())
		.assert()
		.success();
}

#[cfg(unix)]
#[test]
fn test_no_matching_rule_runs_command_unchanged() {
	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");

	fs::write(
		&config_path,
		r#"
root = true

[[rules]]
binary_pattern = ".*/cargo$"
arg_rewrite = "s/build/test/"
"#,
	)
	.unwrap();

	// echo doesn't match .*/cargo$, so it runs unchanged
	tramp_cmd()
		.args(["echo", "build"])
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("build"));
}

// ============================================================================
// Hook tests (Unix only - hooks use shell scripts)
// ============================================================================

#[cfg(unix)]
#[test]
fn test_pre_hook_runs_before_command() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");
	let hook_path = temp_dir.path().join("pre_hook.sh");
	let marker_path = temp_dir.path().join("pre_hook_ran");

	// Create a pre-hook that creates a marker file
	fs::write(
		&hook_path,
		format!("#!/bin/bash\ntouch {}\n", marker_path.to_string_lossy()),
	)
	.unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/echo$"
pre_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	tramp_cmd()
		.args(["echo", "test"])
		.current_dir(temp_dir.path())
		.assert()
		.success();

	assert!(
		marker_path.exists(),
		"Pre-hook should have created marker file"
	);
}

#[cfg(unix)]
#[test]
fn test_pre_hook_failure_aborts_command() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");
	let hook_path = temp_dir.path().join("pre_hook.sh");
	let marker_path = temp_dir.path().join("command_ran");

	// Create a pre-hook that fails
	fs::write(&hook_path, "#!/bin/bash\nexit 1\n").unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/sh$"
pre_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	// This command would create the marker file if it ran
	tramp_cmd()
		.args([
			"sh",
			"-c",
			&format!("touch {}", marker_path.to_string_lossy()),
		])
		.current_dir(temp_dir.path())
		.assert()
		.failure();

	assert!(
		!marker_path.exists(),
		"Command should not have run after pre-hook failure"
	);
}

#[cfg(unix)]
#[test]
fn test_post_hook_runs_after_command() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");
	let hook_path = temp_dir.path().join("post_hook.sh");
	let marker_path = temp_dir.path().join("post_hook_ran");

	// Create a post-hook that creates a marker file
	fs::write(
		&hook_path,
		format!("#!/bin/bash\ntouch {}\n", marker_path.to_string_lossy()),
	)
	.unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/echo$"
post_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	tramp_cmd()
		.args(["echo", "test"])
		.current_dir(temp_dir.path())
		.assert()
		.success();

	assert!(
		marker_path.exists(),
		"Post-hook should have created marker file"
	);
}

#[cfg(unix)]
#[test]
fn test_post_hook_receives_exit_code() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");
	let hook_path = temp_dir.path().join("post_hook.sh");
	let exit_code_file = temp_dir.path().join("exit_code");

	// Create a post-hook that writes the exit code to a file
	fs::write(
		&hook_path,
		format!(
			"#!/bin/bash\necho $TRAMP_EXIT_CODE > {}\n",
			exit_code_file.to_string_lossy()
		),
	)
	.unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/sh$"
post_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	tramp_cmd()
		.args(["sh", "-c", "exit 42"])
		.current_dir(temp_dir.path())
		.assert()
		.code(42);

	let exit_code = fs::read_to_string(&exit_code_file)
		.unwrap()
		.trim()
		.to_string();
	assert_eq!(exit_code, "42");
}

#[cfg(unix)]
#[test]
fn test_intercept_hook_replaces_command() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");
	let hook_path = temp_dir.path().join("intercept_hook.sh");
	let marker_path = temp_dir.path().join("command_ran");

	// Create an intercept hook that exits successfully without running the command
	fs::write(&hook_path, "#!/bin/bash\necho \"intercepted\"\nexit 0\n").unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/sh$"
intercept_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	// This command would create the marker file if it ran
	tramp_cmd()
		.args([
			"sh",
			"-c",
			&format!("touch {}", marker_path.to_string_lossy()),
		])
		.current_dir(temp_dir.path())
		.assert()
		.success()
		.stdout(predicate::str::contains("intercepted"));

	assert!(
		!marker_path.exists(),
		"Original command should not have run"
	);
}

#[cfg(unix)]
#[test]
fn test_intercept_hook_exit_code_propagates() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	let config_path = temp_dir.path().join(".tramp.toml");
	let hook_path = temp_dir.path().join("intercept_hook.sh");

	// Create an intercept hook that exits with code 77
	fs::write(&hook_path, "#!/bin/bash\nexit 77\n").unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/echo$"
intercept_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	tramp_cmd()
		.args(["echo", "test"])
		.current_dir(temp_dir.path())
		.assert()
		.code(77);
}

#[cfg(unix)]
#[test]
fn test_hook_receives_env_vars() {
	use std::os::unix::fs::PermissionsExt;

	let temp_dir = tempfile::tempdir().unwrap();
	// Canonicalize to handle macOS /var -> /private/var symlinks
	let temp_path = temp_dir.path().canonicalize().unwrap();
	let config_path = temp_path.join(".tramp.toml");
	let hook_path = temp_path.join("hook.sh");
	let env_file = temp_path.join("env_vars");

	// Create a hook that writes environment variables to a file
	fs::write(
		&hook_path,
		format!(
			r#"#!/bin/bash
echo "TRAMP_ORIGINAL_ARGS=$TRAMP_ORIGINAL_ARGS" >> {}
echo "TRAMP_CWD=$TRAMP_CWD" >> {}
echo "TRAMP_HOOK_TYPE=$TRAMP_HOOK_TYPE" >> {}
"#,
			env_file.to_string_lossy(),
			env_file.to_string_lossy(),
			env_file.to_string_lossy()
		),
	)
	.unwrap();
	fs::set_permissions(&hook_path, fs::Permissions::from_mode(0o755)).unwrap();

	fs::write(
		&config_path,
		format!(
			r#"
root = true

[[rules]]
binary_pattern = ".*/echo$"
pre_hook = "{}"
"#,
			hook_path.to_string_lossy()
		),
	)
	.unwrap();

	tramp_cmd()
		.args(["echo", "arg1", "arg2"])
		.current_dir(&temp_path)
		.assert()
		.success();

	let env_content = fs::read_to_string(&env_file).unwrap();
	assert!(env_content.contains("TRAMP_ORIGINAL_ARGS=arg1 arg2"));
	assert!(env_content.contains(&format!("TRAMP_CWD={}", temp_path.to_string_lossy())));
	assert!(env_content.contains("TRAMP_HOOK_TYPE=pre"));
}
