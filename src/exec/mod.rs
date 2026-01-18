//! Command execution for tramp.
//!
//! This module handles:
//! - Executing wrapped commands with proper stdio handling
//! - Exit code propagation
//! - Trampoline script generation

pub mod trampoline;

use crate::error::{Result, TrampError};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

/// Execute a command with proper stdio handling.
///
/// This function:
/// - Passes stdin, stdout, stderr through to the child process
/// - Returns the exit status of the child process
pub fn execute_command(binary: &Path, args: &[String], cwd: &Path) -> Result<ExitStatus> {
	let mut cmd = Command::new(binary);
	cmd.args(args)
		.current_dir(cwd)
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit());

	let status = cmd.status().map_err(|source| {
		if source.kind() == std::io::ErrorKind::NotFound {
			TrampError::CommandNotFound {
				command: binary.to_string_lossy().to_string(),
			}
		} else {
			TrampError::CommandFailed {
				command: binary.to_string_lossy().to_string(),
				source,
			}
		}
	})?;

	Ok(status)
}

/// Resolve a command name to its full path.
///
/// If the command is already an absolute path, returns it as-is.
/// Otherwise, searches PATH for the command.
pub fn resolve_command(command: &str) -> Option<std::path::PathBuf> {
	let path = Path::new(command);

	// If it's already an absolute path, return it if it exists
	if path.is_absolute() {
		if path.exists() {
			return Some(path.to_path_buf());
		} else {
			return None;
		}
	}

	// Search PATH
	if let Ok(path_var) = std::env::var("PATH") {
		for dir in std::env::split_paths(&path_var) {
			let full_path = dir.join(command);
			if full_path.exists() {
				return Some(full_path);
			}
		}
	}

	None
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_resolve_command_absolute_path() {
		// /bin/sh should exist on Unix systems
		#[cfg(unix)]
		{
			let result = resolve_command("/bin/sh");
			assert!(result.is_some());
			assert_eq!(result.unwrap(), Path::new("/bin/sh"));
		}
	}

	#[test]
	fn test_resolve_command_not_found() {
		let result = resolve_command("/nonexistent/path/to/binary");
		assert!(result.is_none());
	}

	#[test]
	fn test_resolve_command_from_path() {
		// 'sh' should be in PATH on Unix systems
		#[cfg(unix)]
		{
			let result = resolve_command("sh");
			assert!(result.is_some());
		}
	}
}
