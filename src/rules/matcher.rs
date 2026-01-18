use crate::config::types::{MergedConfig, Rule, RuleWithSource};
use crate::error::{Result, TrampError};
use regex::Regex;
use std::path::Path;

/// Context for matching rules against a command invocation.
#[derive(Debug)]
pub struct MatchContext<'a> {
	/// The binary path being executed.
	pub binary_path: &'a Path,

	/// The current working directory.
	pub cwd: &'a Path,

	/// The arguments passed to the command.
	pub args: &'a [String],
}

/// A compiled rule ready for matching.
#[derive(Debug)]
pub struct CompiledRule {
	/// The original rule.
	pub rule: Rule,

	/// Compiled binary pattern regex.
	pub binary_regex: Option<Regex>,

	/// Compiled cwd pattern regex.
	pub cwd_regex: Option<Regex>,

	/// Source config path (for debugging).
	pub source: std::path::PathBuf,
}

impl CompiledRule {
	/// Compile a rule from a RuleWithSource.
	pub fn from_rule_with_source(rws: &RuleWithSource) -> Result<Self> {
		let binary_regex = rws
			.rule
			.binary_pattern
			.as_ref()
			.map(|p| compile_regex(p))
			.transpose()?;

		let cwd_regex = rws
			.rule
			.cwd_pattern
			.as_ref()
			.map(|p| compile_regex(p))
			.transpose()?;

		Ok(CompiledRule {
			rule: rws.rule.clone(),
			binary_regex,
			cwd_regex,
			source: rws.source.clone(),
		})
	}

	/// Check if this rule matches the given context.
	pub fn matches(&self, ctx: &MatchContext) -> bool {
		// Check binary pattern if specified
		if let Some(ref regex) = self.binary_regex {
			let binary_str = ctx.binary_path.to_string_lossy();
			if !regex.is_match(&binary_str) {
				return false;
			}
		}

		// Check cwd pattern if specified
		if let Some(ref regex) = self.cwd_regex {
			let cwd_str = ctx.cwd.to_string_lossy();
			if !regex.is_match(&cwd_str) {
				return false;
			}
		}

		true
	}
}

/// Compile a regex pattern string.
fn compile_regex(pattern: &str) -> Result<Regex> {
	Regex::new(pattern).map_err(|source| TrampError::InvalidRegex {
		pattern: pattern.to_string(),
		source,
	})
}

/// Compile all rules in a merged config.
pub fn compile_rules(config: &MergedConfig) -> Result<Vec<CompiledRule>> {
	config
		.rules
		.iter()
		.map(CompiledRule::from_rule_with_source)
		.collect()
}

/// Find the first matching rule for a given context.
pub fn find_matching_rule<'a>(
	rules: &'a [CompiledRule],
	ctx: &MatchContext,
) -> Option<&'a CompiledRule> {
	rules.iter().find(|rule| rule.matches(ctx))
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::PathBuf;

	fn make_rule_with_source(rule: Rule) -> RuleWithSource {
		RuleWithSource {
			rule,
			source: PathBuf::from("test.toml"),
		}
	}

	#[test]
	fn test_compile_valid_regex() {
		let result = compile_regex(r".*/cargo$");
		assert!(result.is_ok());
	}

	#[test]
	fn test_compile_invalid_regex() {
		let result = compile_regex(r"[invalid");
		assert!(result.is_err());
		match result.unwrap_err() {
			TrampError::InvalidRegex { pattern, .. } => {
				assert_eq!(pattern, "[invalid");
			}
			_ => panic!("Expected InvalidRegex error"),
		}
	}

	#[test]
	fn test_rule_matches_binary_pattern() {
		let rule = Rule {
			binary_pattern: Some(r".*/cargo$".to_string()),
			..Default::default()
		};
		let rws = make_rule_with_source(rule);
		let compiled = CompiledRule::from_rule_with_source(&rws).unwrap();

		let ctx = MatchContext {
			binary_path: Path::new("/usr/local/bin/cargo"),
			cwd: Path::new("/home/user/project"),
			args: &[],
		};
		assert!(compiled.matches(&ctx));

		let ctx_no_match = MatchContext {
			binary_path: Path::new("/usr/local/bin/rustc"),
			cwd: Path::new("/home/user/project"),
			args: &[],
		};
		assert!(!compiled.matches(&ctx_no_match));
	}

	#[test]
	fn test_rule_matches_cwd_pattern() {
		let rule = Rule {
			cwd_pattern: Some(r".*/my-project$".to_string()),
			..Default::default()
		};
		let rws = make_rule_with_source(rule);
		let compiled = CompiledRule::from_rule_with_source(&rws).unwrap();

		let ctx = MatchContext {
			binary_path: Path::new("/usr/local/bin/cargo"),
			cwd: Path::new("/home/user/my-project"),
			args: &[],
		};
		assert!(compiled.matches(&ctx));

		let ctx_no_match = MatchContext {
			binary_path: Path::new("/usr/local/bin/cargo"),
			cwd: Path::new("/home/user/other-project"),
			args: &[],
		};
		assert!(!compiled.matches(&ctx_no_match));
	}

	#[test]
	fn test_rule_matches_both_patterns() {
		let rule = Rule {
			binary_pattern: Some(r".*/cargo$".to_string()),
			cwd_pattern: Some(r".*/my-project$".to_string()),
			..Default::default()
		};
		let rws = make_rule_with_source(rule);
		let compiled = CompiledRule::from_rule_with_source(&rws).unwrap();

		// Both match
		let ctx = MatchContext {
			binary_path: Path::new("/usr/local/bin/cargo"),
			cwd: Path::new("/home/user/my-project"),
			args: &[],
		};
		assert!(compiled.matches(&ctx));

		// Binary matches, cwd doesn't
		let ctx_cwd_mismatch = MatchContext {
			binary_path: Path::new("/usr/local/bin/cargo"),
			cwd: Path::new("/home/user/other-project"),
			args: &[],
		};
		assert!(!compiled.matches(&ctx_cwd_mismatch));

		// Cwd matches, binary doesn't
		let ctx_binary_mismatch = MatchContext {
			binary_path: Path::new("/usr/local/bin/rustc"),
			cwd: Path::new("/home/user/my-project"),
			args: &[],
		};
		assert!(!compiled.matches(&ctx_binary_mismatch));
	}

	#[test]
	fn test_rule_with_no_patterns_matches_everything() {
		let rule = Rule::default();
		let rws = make_rule_with_source(rule);
		let compiled = CompiledRule::from_rule_with_source(&rws).unwrap();

		let ctx = MatchContext {
			binary_path: Path::new("/any/path"),
			cwd: Path::new("/any/dir"),
			args: &[],
		};
		assert!(compiled.matches(&ctx));
	}

	#[test]
	fn test_find_matching_rule_first_wins() {
		let rules = vec![
			Rule {
				binary_pattern: Some(r".*/cargo$".to_string()),
				arg_rewrite: Some("s/build/build --release/".to_string()),
				..Default::default()
			},
			Rule {
				binary_pattern: Some(r".*/cargo$".to_string()),
				arg_rewrite: Some("s/build/build --debug/".to_string()),
				..Default::default()
			},
		];

		let rules_with_source: Vec<_> = rules.into_iter().map(make_rule_with_source).collect();
		let compiled: Vec<_> = rules_with_source
			.iter()
			.map(|r| CompiledRule::from_rule_with_source(r).unwrap())
			.collect();

		let ctx = MatchContext {
			binary_path: Path::new("/usr/local/bin/cargo"),
			cwd: Path::new("/home/user/project"),
			args: &[],
		};

		let matched = find_matching_rule(&compiled, &ctx);
		assert!(matched.is_some());
		assert_eq!(
			matched.unwrap().rule.arg_rewrite,
			Some("s/build/build --release/".to_string())
		);
	}
}
