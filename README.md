# Tramp

A CLI tool for proxying commands with pre/post hooks and trampolines.

## Overview

**Tramp** (short for Trampoline) wraps command execution with configurable hooks, enabling:

- **Pre-hooks**: Rewrite CLI commands before execution
- **Post-hooks**: Process or transform command output
- **Intercept hooks**: Replace command execution entirely
- **Trampolines**: Ensure tools are built before first use

## Use Cases

### Local Development Customization

A company project has commands that do side effects like updating resources in a parallel directory. Developers might need to customize behavior based on their local system—working with multiple worktrees, or limited disk space. Tramp makes it easy to customize behavior with pre-hooks that rewrite CLI commands and post-hooks that transform output.

### Build Instrumentation

A developer wants specific events in a build process to trigger something elsewhere—for profiling, logging, or integration purposes. Rather than post-processing entire build logs, tramp lets you instrument particular commands. This instrumentation can be machine-specific or temporarily enabled.

### Workflow Logging

A developer wants to extract particular events from their workflow to their daily worklog for standup meetings.

## Installation

```bash
# From crates.io
cargo install tramp

# From GitHub
cargo install --git https://github.com/fbartho/tramp
```

## Usage

```bash
# Run a command through tramp
tramp <command> [args...]

# Generate a trampoline script
tramp --setup ./my-binary

# Create a template .tramp.toml in current directory
tramp --init

# Overwrite existing .tramp.toml
tramp --init --force
```

## Configuration

Tramp uses `.tramp.toml` files with directory cascade:

1. Look for `.tramp.toml` in current directory
2. Read and apply rules
3. Continue up directory tree unless `root = true`
4. Finally check `~/.tramp.toml`

### Example Configuration

```toml
# Stop the cascade and jump to user config
root = true

# Disable external hooks (for high-sensitivity situations)
no-external-lookup = true

# Skip user config when this env var is truthy (for CI)
root-config-lookup-disable-env-var = "CI"

# Rules: first matching rule wins
# Option A: Array of tables syntax
[[rules]]
binary_pattern = ".*/cargo$"
arg_rewrite = "s/^build$/build --release/"
pre_hook = "/path/to/pre-hook.sh"

[[rules]]
cwd_pattern = ".*/my-project$"
binary_pattern = ".*/npm$"
alternate_command = "/usr/local/bin/pnpm"
```

Or use inline table syntax (both parse identically):

```toml
root = true

# Option B: Inline tables syntax
*TOML parsers may complain if you use `#` comments inside one of th e rules*

rules = [
  { binary_pattern = ".*/cargo$", arg_rewrite = "s/^build$/build --release/", pre_hook = "/path/to/pre-hook.sh" },
  { cwd_pattern = ".*/my-project$", binary_pattern = ".*/npm$", alternate_command = "/usr/local/bin/pnpm" },
  { binary_pattern = ".*/kubectl$", command_rewrite = "s/kubectl/kubectl --context=dev/" },
]
```

## Features

- Pipes, stdin, stderr, and exit codes propagate correctly
- Config file cascade with merge semantics
- First matching rule wins
- Binary matching via regex
- Working directory matching via regex
- Argument rewriting via regex
- Full command rewriting via regex
- Alternate command substitution

## License

Apache 2.0 - See [LICENSE](LICENSE)
