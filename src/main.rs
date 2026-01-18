use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tramp_cli::config::{load_merged_config, user_config_path};
use tramp_cli::exec::trampoline::{generate_init_template, generate_trampoline_script};
use tramp_cli::exec::{execute_command, resolve_command};
use tramp_cli::hooks::{
	HookContext, HookType, execute_intercept_hook, execute_post_hook, execute_pre_hook,
};
use tramp_cli::rules::{
	MatchContext, Substitution, compile_rules, find_matching_rule, rewrite_args, rewrite_command,
};

#[derive(Parser)]
#[command(name = "tramp")]
#[command(
	author,
	version,
	about = "CLI tool for proxying commands with pre/post hooks and trampolines"
)]
#[command(arg_required_else_help = true)]
struct Cli {
	#[command(subcommand)]
	command: Option<Commands>,

	/// Generate a trampoline wrapper script for the given binary
	#[arg(long, value_name = "BINARY")]
	setup: Option<PathBuf>,

	/// Create a template .tramp.toml in the current directory
	#[arg(long)]
	init: bool,

	/// Overwrite existing .tramp.toml when using --init
	#[arg(long, requires = "init")]
	force: bool,

	/// Command to run through tramp
	#[arg(trailing_var_arg = true, allow_hyphen_values = true)]
	args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
	/// Configuration management commands
	Config {
		#[command(subcommand)]
		action: ConfigAction,
	},
}

#[derive(Subcommand)]
enum ConfigAction {
	/// Display merged effective configuration with source annotations
	Show,
	/// Check all config files for errors without running anything
	Validate,
}

fn main() -> ExitCode {
	match run() {
		Ok(code) => code,
		Err(e) => {
			eprintln!("error: {e:?}");
			ExitCode::FAILURE
		}
	}
}

fn run() -> Result<ExitCode> {
	let cli = Cli::parse();

	// Handle --setup
	if let Some(binary_path) = cli.setup {
		return handle_setup(&binary_path);
	}

	// Handle --init
	if cli.init {
		return handle_init(cli.force);
	}

	// Handle subcommands
	if let Some(command) = cli.command {
		return match command {
			Commands::Config { action } => match action {
				ConfigAction::Show => handle_config_show(),
				ConfigAction::Validate => handle_config_validate(),
			},
		};
	}

	// Handle command execution
	if !cli.args.is_empty() {
		return handle_command(&cli.args);
	}

	// No command specified - this shouldn't happen due to arg_required_else_help
	Ok(ExitCode::SUCCESS)
}

fn handle_setup(binary_path: &Path) -> Result<ExitCode> {
	let script = generate_trampoline_script(binary_path, None);
	println!("{}", script);
	Ok(ExitCode::SUCCESS)
}

fn handle_init(force: bool) -> Result<ExitCode> {
	let config_path = PathBuf::from(".tramp.toml");

	if config_path.exists() && !force {
		anyhow::bail!(".tramp.toml already exists. Use --force to overwrite.");
	}

	let template = generate_init_template();
	std::fs::write(&config_path, template)
		.with_context(|| format!("Failed to write {}", config_path.display()))?;

	println!("Created .tramp.toml");
	Ok(ExitCode::SUCCESS)
}

fn handle_config_show() -> Result<ExitCode> {
	let cwd = std::env::current_dir().context("Failed to get current directory")?;
	let configs =
		tramp_cli::config::discover_configs(&cwd).context("Failed to discover config files")?;

	if configs.is_empty() {
		println!("No configuration files found.");
		return Ok(ExitCode::SUCCESS);
	}

	println!("Configuration files (in cascade order):\n");

	for loaded in &configs {
		println!("# Source: {}", loaded.path.display());
		println!("# root: {}", loaded.config.root);
		println!("# no-external-lookup: {}", loaded.config.no_external_lookup);
		if let Some(ref env_var) = loaded.config.root_config_lookup_disable_env_var {
			println!("# root-config-lookup-disable-env-var: {}", env_var);
		}
		println!("# rules: {}", loaded.config.rules.len());
		println!();

		for (i, rule) in loaded.config.rules.iter().enumerate() {
			println!("  Rule {}:", i + 1);
			if let Some(ref pattern) = rule.binary_pattern {
				println!("    binary_pattern: {}", pattern);
			}
			if let Some(ref pattern) = rule.cwd_pattern {
				println!("    cwd_pattern: {}", pattern);
			}
			if let Some(ref rewrite) = rule.arg_rewrite {
				println!("    arg_rewrite: {}", rewrite);
			}
			if let Some(ref rewrite) = rule.command_rewrite {
				println!("    command_rewrite: {}", rewrite);
			}
			if let Some(ref cmd) = rule.alternate_command {
				println!("    alternate_command: {}", cmd);
			}
			if let Some(ref hook) = rule.pre_hook {
				println!("    pre_hook: {}", hook.display());
			}
			if let Some(ref hook) = rule.post_hook {
				println!("    post_hook: {}", hook.display());
			}
			if let Some(ref hook) = rule.intercept_hook {
				println!("    intercept_hook: {}", hook.display());
			}
			println!();
		}
	}

	// Show user config path
	if let Ok(user_path) = user_config_path() {
		println!("User config path: {}", user_path.display());
		if user_path.exists() {
			println!("  (exists)");
		} else {
			println!("  (not found)");
		}
	}

	Ok(ExitCode::SUCCESS)
}

fn handle_config_validate() -> Result<ExitCode> {
	let cwd = std::env::current_dir().context("Failed to get current directory")?;

	match tramp_cli::config::discover_configs(&cwd) {
		Ok(configs) => {
			if configs.is_empty() {
				println!("No configuration files found.");
			} else {
				println!("All configuration files are valid:");
				for loaded in &configs {
					println!(
						"  {} ({} rules)",
						loaded.path.display(),
						loaded.config.rules.len()
					);
				}
			}
			Ok(ExitCode::SUCCESS)
		}
		Err(e) => {
			eprintln!("Configuration error: {}", e);
			Ok(ExitCode::FAILURE)
		}
	}
}

fn handle_command(args: &[String]) -> Result<ExitCode> {
	let command_name = &args[0];
	let command_args: Vec<String> = args[1..].to_vec();

	let cwd = std::env::current_dir().context("Failed to get current directory")?;

	// Resolve command to full path
	let binary_path = resolve_command(command_name)
		.ok_or_else(|| anyhow::anyhow!("Command not found: {}", command_name))?;

	// Load and merge config
	let config = load_merged_config(&cwd).context("Failed to load configuration")?;

	// Compile rules
	let rules = compile_rules(&config).context("Failed to compile rules")?;

	// Create match context
	let ctx = MatchContext {
		binary_path: &binary_path,
		cwd: &cwd,
		args: &command_args,
	};

	// Find matching rule
	let matched_rule = find_matching_rule(&rules, &ctx);

	// Determine final binary and args
	let (final_binary, final_args) = if let Some(rule) = matched_rule {
		apply_rule(&binary_path, &command_args, &rule.rule)?
	} else {
		(binary_path.clone(), command_args.clone())
	};

	// Execute pre-hook if present
	if let Some(rule) = matched_rule {
		if let Some(ref pre_hook) = rule.rule.pre_hook {
			let hook_ctx = HookContext {
				original_binary: &binary_path,
				original_args: &command_args,
				cwd: &cwd,
				hook_type: HookType::Pre,
				executed_binary: None,
				executed_args: None,
				exit_code: None,
			};
			execute_pre_hook(pre_hook, &hook_ctx)
				.with_context(|| format!("Pre-hook failed: {}", pre_hook.display()))?;
		}

		// Check for intercept hook
		if let Some(ref intercept_hook) = rule.rule.intercept_hook {
			let hook_ctx = HookContext {
				original_binary: &binary_path,
				original_args: &command_args,
				cwd: &cwd,
				hook_type: HookType::Intercept,
				executed_binary: Some(&final_binary),
				executed_args: Some(&final_args),
				exit_code: None,
			};
			let exit_code = execute_intercept_hook(intercept_hook, &hook_ctx)
				.with_context(|| format!("Intercept hook failed: {}", intercept_hook.display()))?;
			return Ok(ExitCode::from(exit_code as u8));
		}
	}

	// Execute the command
	let status = execute_command(&final_binary, &final_args, &cwd)
		.with_context(|| format!("Failed to execute: {}", final_binary.display()))?;

	let exit_code = status.code().unwrap_or(1);

	// Execute post-hook if present
	if let Some(rule) = matched_rule
		&& let Some(ref post_hook) = rule.rule.post_hook
	{
		let hook_ctx = HookContext {
			original_binary: &binary_path,
			original_args: &command_args,
			cwd: &cwd,
			hook_type: HookType::Post,
			executed_binary: Some(&final_binary),
			executed_args: Some(&final_args),
			exit_code: Some(exit_code),
		};
		// Post-hooks don't fail the command, just log if they error
		if let Err(e) = execute_post_hook(post_hook, &hook_ctx) {
			eprintln!("Warning: post-hook failed: {}", e);
		}
	}

	Ok(ExitCode::from(exit_code as u8))
}

fn apply_rule(
	binary_path: &Path,
	args: &[String],
	rule: &tramp_cli::config::Rule,
) -> Result<(PathBuf, Vec<String>)> {
	// Check for alternate command
	if let Some(ref alternate) = rule.alternate_command {
		let alt_path = resolve_command(alternate)
			.ok_or_else(|| anyhow::anyhow!("Alternate command not found: {}", alternate))?;
		return Ok((alt_path, args.to_vec()));
	}

	// Check for arg_rewrite
	if let Some(ref rewrite) = rule.arg_rewrite {
		let sub = Substitution::parse(rewrite)
			.with_context(|| format!("Invalid arg_rewrite pattern: {}", rewrite))?;
		let new_args = rewrite_args(args, &sub);
		return Ok((binary_path.to_path_buf(), new_args));
	}

	// Check for command_rewrite
	if let Some(ref rewrite) = rule.command_rewrite {
		let sub = Substitution::parse(rewrite)
			.with_context(|| format!("Invalid command_rewrite pattern: {}", rewrite))?;
		let binary_str = binary_path.to_string_lossy();
		let (new_binary_str, new_args) = rewrite_command(&binary_str, args, &sub);
		let new_binary = resolve_command(&new_binary_str)
			.ok_or_else(|| anyhow::anyhow!("Rewritten command not found: {}", new_binary_str))?;
		return Ok((new_binary, new_args));
	}

	// No rewrite, return as-is
	Ok((binary_path.to_path_buf(), args.to_vec()))
}
