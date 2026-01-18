use crate::error::{Result, TrampError};
use regex::Regex;

/// Parsed substitution command (sed-like syntax).
#[derive(Debug)]
pub struct Substitution {
	/// The pattern to match.
	pub pattern: Regex,

	/// The replacement string.
	pub replacement: String,

	/// Whether to replace all occurrences (global flag).
	pub global: bool,
}

impl Substitution {
	/// Parse a substitution string in sed-like format: "s/pattern/replacement/" or "s/pattern/replacement/g"
	pub fn parse(input: &str) -> Result<Self> {
		// Must start with 's'
		if !input.starts_with('s') {
			return Err(TrampError::InvalidRegex {
				pattern: input.to_string(),
				source: regex::Error::Syntax("Substitution must start with 's'".to_string()),
			});
		}

		// Get the delimiter (character after 's')
		let chars: Vec<char> = input.chars().collect();
		if chars.len() < 2 {
			return Err(TrampError::InvalidRegex {
				pattern: input.to_string(),
				source: regex::Error::Syntax("Substitution too short".to_string()),
			});
		}

		let delimiter = chars[1];

		// Split by delimiter, handling escapes
		let parts = split_by_delimiter(&input[2..], delimiter)?;

		if parts.len() < 2 {
			return Err(TrampError::InvalidRegex {
				pattern: input.to_string(),
				source: regex::Error::Syntax(
					"Substitution must have pattern and replacement".to_string(),
				),
			});
		}

		let pattern_str = &parts[0];
		let replacement = parts[1].clone();
		let flags = if parts.len() > 2 { &parts[2] } else { "" };

		let global = flags.contains('g');

		let pattern = Regex::new(pattern_str).map_err(|source| TrampError::InvalidRegex {
			pattern: pattern_str.to_string(),
			source,
		})?;

		Ok(Substitution {
			pattern,
			replacement,
			global,
		})
	}

	/// Apply this substitution to a string.
	pub fn apply(&self, input: &str) -> String {
		if self.global {
			self.pattern
				.replace_all(input, &self.replacement)
				.to_string()
		} else {
			self.pattern.replace(input, &self.replacement).to_string()
		}
	}
}

/// Split a string by a delimiter, respecting backslash escapes.
fn split_by_delimiter(input: &str, delimiter: char) -> Result<Vec<String>> {
	let mut parts = Vec::new();
	let mut current = String::new();
	let mut chars = input.chars().peekable();
	let mut escape_next = false;

	while let Some(c) = chars.next() {
		if escape_next {
			current.push(c);
			escape_next = false;
		} else if c == '\\' {
			// Check if we're escaping the delimiter
			if chars.peek() == Some(&delimiter) {
				escape_next = true;
			} else {
				current.push(c);
			}
		} else if c == delimiter {
			parts.push(current);
			current = String::new();
		} else {
			current.push(c);
		}
	}

	// Add the last part
	parts.push(current);

	Ok(parts)
}

/// Rewrite arguments using a substitution.
pub fn rewrite_args(args: &[String], substitution: &Substitution) -> Vec<String> {
	// Join args with space, apply substitution, split back
	let args_str = args.join(" ");
	let rewritten = substitution.apply(&args_str);

	// Split back into args (simple split by whitespace)
	// Note: This doesn't handle quoted arguments perfectly, but matches the sed-like behavior
	rewritten
		.split_whitespace()
		.map(|s| s.to_string())
		.collect()
}

/// Rewrite the entire command (binary + args) using a substitution.
pub fn rewrite_command(
	binary: &str,
	args: &[String],
	substitution: &Substitution,
) -> (String, Vec<String>) {
	// Join binary and args
	let mut command_parts = vec![binary.to_string()];
	command_parts.extend(args.iter().cloned());
	let command_str = command_parts.join(" ");

	// Apply substitution
	let rewritten = substitution.apply(&command_str);

	// Split back into binary and args
	let mut parts: Vec<String> = rewritten
		.split_whitespace()
		.map(|s| s.to_string())
		.collect();

	if parts.is_empty() {
		(binary.to_string(), vec![])
	} else {
		let new_binary = parts.remove(0);
		(new_binary, parts)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_simple_substitution() {
		let sub = Substitution::parse("s/foo/bar/").unwrap();
		assert_eq!(sub.replacement, "bar");
		assert!(!sub.global);
	}

	#[test]
	fn test_parse_global_substitution() {
		let sub = Substitution::parse("s/foo/bar/g").unwrap();
		assert_eq!(sub.replacement, "bar");
		assert!(sub.global);
	}

	#[test]
	fn test_parse_different_delimiter() {
		let sub = Substitution::parse("s#foo#bar#").unwrap();
		assert_eq!(sub.replacement, "bar");
	}

	#[test]
	fn test_parse_escaped_delimiter() {
		let sub = Substitution::parse(r"s/foo\/bar/baz/").unwrap();
		assert_eq!(sub.replacement, "baz");
		assert_eq!(sub.apply("foo/bar"), "baz");
	}

	#[test]
	fn test_apply_substitution() {
		let sub = Substitution::parse("s/build/build --release/").unwrap();
		assert_eq!(sub.apply("build"), "build --release");
	}

	#[test]
	fn test_apply_global_substitution() {
		let sub = Substitution::parse("s/foo/bar/g").unwrap();
		assert_eq!(sub.apply("foo foo foo"), "bar bar bar");
	}

	#[test]
	fn test_apply_non_global_substitution() {
		let sub = Substitution::parse("s/foo/bar/").unwrap();
		assert_eq!(sub.apply("foo foo foo"), "bar foo foo");
	}

	#[test]
	fn test_apply_with_capture_groups() {
		let sub = Substitution::parse(r"s/(\w+)/[$1]/").unwrap();
		assert_eq!(sub.apply("hello world"), "[hello] world");
	}

	#[test]
	fn test_apply_with_capture_groups_global() {
		let sub = Substitution::parse(r"s/(\w+)/[$1]/g").unwrap();
		assert_eq!(sub.apply("hello world"), "[hello] [world]");
	}

	#[test]
	fn test_rewrite_args() {
		let sub = Substitution::parse("s/^build$/build --release/").unwrap();
		let args = vec!["build".to_string()];
		let rewritten = rewrite_args(&args, &sub);
		assert_eq!(rewritten, vec!["build", "--release"]);
	}

	#[test]
	fn test_rewrite_command() {
		let sub = Substitution::parse("s/kubectl/kubectl --context=dev/").unwrap();
		let (binary, args) =
			rewrite_command("kubectl", &["get".to_string(), "pods".to_string()], &sub);
		assert_eq!(binary, "kubectl");
		assert_eq!(args, vec!["--context=dev", "get", "pods"]);
	}

	#[test]
	fn test_invalid_substitution_no_s() {
		let result = Substitution::parse("foo/bar/");
		assert!(result.is_err());
	}

	#[test]
	fn test_invalid_substitution_too_short() {
		let result = Substitution::parse("s");
		assert!(result.is_err());
	}
}
