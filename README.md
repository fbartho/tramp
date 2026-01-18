# Tramp

A CLI tool for proxying commands with pre/post hooks and trampolines.

## Overview

**`tramp`** (short for Trampoline) wraps command execution with configurable hooks, enabling:

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
# From crates.io (installs as 'tramp')
cargo install tramp-cli

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

# Intercept hook: completely replace command execution
[[rules]]
binary_pattern = ".*/my-custom-tool$"
intercept_hook = "/path/to/intercept.sh"
```

Or use inline table syntax (both parse identically):

```toml
root = true

# Option B: Inline tables syntax
*TOML parsers may complain if you use `#` comments inside one of the rules*

rules = [
  { binary_pattern = ".*/cargo$", arg_rewrite = "s/^build$/build --release/", pre_hook = "/path/to/pre-hook.sh" },
  { cwd_pattern = ".*/my-project$", binary_pattern = ".*/npm$", alternate_command = "/usr/local/bin/pnpm" },
  { binary_pattern = ".*/kubectl$", command_rewrite = "s/kubectl/kubectl --context=dev/" },
]
```

## Hook Types

### Pre-hooks

Run before the command executes. If a pre-hook exits with non-zero, the command is aborted.

```toml
[[rules]]
binary_pattern = ".*/cargo$"
pre_hook = "/path/to/pre-hook.sh"
```

### Post-hooks

Run after the command completes. Receive `TRAMP_EXIT_CODE` with the command's exit status.

```toml
[[rules]]
binary_pattern = ".*/cargo$"
post_hook = "/path/to/post-hook.sh"
```

### Intercept hooks

**Completely replace** command execution. The original command never runs—the intercept hook runs instead. Useful for:
- Mocking commands in development/testing
- Implementing custom command behavior
- Redirecting commands to different implementations

```toml
[[rules]]
binary_pattern = ".*/deploy$"
intercept_hook = "/path/to/intercept.sh"
```

**Example intercept hook:**

```bash
#!/bin/bash
# intercept.sh - Replace 'deploy' with a dry-run in development

echo "Intercepted: $TRAMP_ORIGINAL_BINARY $TRAMP_ORIGINAL_ARGS"
echo "Would deploy from $TRAMP_CWD"
echo "(Dry run - no actual deployment)"
exit 0
```

The intercept hook's exit code becomes tramp's exit code.

## Hook Environment Variables

When hooks execute, tramp provides context via environment variables:

| Variable | Description |
|----------|-------------|
| `TRAMP_ORIGINAL_BINARY` | Path to the original command |
| `TRAMP_ORIGINAL_ARGS` | Original arguments as a string |
| `TRAMP_ORIGINAL_ARG_N` | Individual arguments (0-indexed) |
| `TRAMP_CWD` | Working directory |
| `TRAMP_HOOK_TYPE` | `pre`, `post`, or `intercept` |
| `TRAMP_EXIT_CODE` | Exit code (post-hooks only) |

**Example hook:**

```bash
#!/bin/bash
# Log failed builds
if [[ "$TRAMP_EXIT_CODE" != "0" ]]; then
    echo "Build failed in $TRAMP_CWD" >> ~/.build-failures.log
fi
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

## Security Considerations

Tramp executes hooks defined in `.tramp.toml` configuration files. When working in untrusted directories (e.g., cloned repositories from unknown sources), be aware that a malicious `.tramp.toml` could execute arbitrary code.

**Recommendations:**
- Review `.tramp.toml` files in new projects before running tramp
- Use `no-external-lookup = true` in your `~/.tramp.toml` to prevent local configs from overriding your hooks
- In CI environments, use `root-config-lookup-disable-env-var = "CI"` to skip user configs

## License

Apache 2.0 - See [LICENSE](LICENSE)
