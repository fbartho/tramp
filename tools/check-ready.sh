#!/bin/bash
# check-ready.sh - Validate code before commit or signaling ready_for_review
#
# Usage: ./tools/check-ready.sh [--fix] [--verbose]
#
# Run this before committing. It validates:
# 1. Code is formatted (runs cargo fmt)
# 2. No uncommitted changes (clean working tree)
# 3. Cargo build passes
# 4. Cargo tests pass
# 5. Clippy passes (no warnings)
#
# Options:
#   --fix      Attempt to auto-fix formatting before checking
#   --verbose  Show full output for all checks, not just failures
#
# Exit codes:
#   0 - All checks passed, safe to commit
#   1 - Checks failed (see output for issues to fix)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Track failures
FAILURES=()

# Parse arguments
FIX_MODE=false
VERBOSE=false
for arg in "$@"; do
    case $arg in
        --fix)
            FIX_MODE=true
            ;;
        --verbose)
            VERBOSE=true
            ;;
    esac
done

log_check() { echo -e "${BLUE}[CHECK]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; FAILURES+=("$1"); }
log_skip() { echo -e "${YELLOW}[SKIP]${NC} $1"; }
log_info() { echo -e "${YELLOW}[INFO]${NC} $1"; }

# Run a command and check exit code properly
# Usage: run_check "description" "fail_message" command args...
# On success: logs [PASS], shows output only if --verbose
# On failure: logs [FAIL], always shows output
run_check() {
    local description="$1"
    local fail_message="$2"
    shift 2

    log_check "$description"

    local output
    local exit_code
    output=$("$@" 2>&1) && exit_code=0 || exit_code=$?

    if [[ $exit_code -eq 0 ]]; then
        log_pass "$description"
        if [[ "$VERBOSE" == true && -n "$output" ]]; then
            echo "$output" | tail -10
        fi
        return 0
    else
        log_fail "$fail_message"
        echo ""
        echo "$output" | tail -30
        echo ""
        return 1
    fi
}

cd "$REPO_ROOT"

echo "========================================"
echo "Pre-Commit Check"
echo "========================================"
echo ""

# 1. Format code first
if [[ -f "Cargo.toml" ]]; then
    if [[ "$FIX_MODE" == true ]]; then
        log_info "Running cargo fmt..."
        cargo fmt 2>/dev/null || true
    fi

    log_check "Checking code formatting (cargo fmt --check)..."
    FORMAT_OUTPUT=$(cargo fmt --check 2>&1) && FORMAT_EXIT=0 || FORMAT_EXIT=$?
    if [[ $FORMAT_EXIT -eq 0 ]]; then
        log_pass "Code formatted correctly"
    else
        log_fail "Code not formatted (run 'cargo fmt' to fix)"
        echo ""
        echo "$FORMAT_OUTPUT" | head -20
        echo ""
    fi
else
    log_skip "No Cargo.toml found, skipping format check"
fi

# 2. Check for uncommitted changes (after formatting)
log_check "Checking for uncommitted changes..."
if [[ -n $(git status --porcelain) ]]; then
    log_fail "Uncommitted changes detected"
    echo ""
    echo "Uncommitted files:"
    git status --short
    echo ""
else
    log_pass "Working tree is clean"
fi

# 3. Rust checks (if Cargo.toml exists)
if [[ -f "Cargo.toml" ]]; then
    # Build
    run_check "Cargo build" "Cargo build failed" cargo build || true

    # Tests
    run_check "Cargo tests" "Cargo tests failed" cargo test || true

    # Clippy
    if command -v cargo &> /dev/null; then
        log_check "Clippy"
        CLIPPY_OUTPUT=$(cargo clippy -- -D warnings 2>&1) && CLIPPY_EXIT=0 || CLIPPY_EXIT=$?
        if [[ $CLIPPY_EXIT -eq 0 ]]; then
            log_pass "Clippy (no warnings)"
            if [[ "$VERBOSE" == true ]]; then
                echo "$CLIPPY_OUTPUT" | tail -5
            fi
        else
            log_fail "Clippy warnings/errors found"
            echo ""
            echo "$CLIPPY_OUTPUT" | grep -E "^error|warning:" | head -20
            echo ""
        fi
    fi
else
    log_skip "No Cargo.toml found, skipping Rust checks"
fi

# Summary
echo ""
echo "========================================"
echo "Summary"
echo "========================================"

if [[ ${#FAILURES[@]} -eq 0 ]]; then
    echo -e "${GREEN}All checks passed!${NC}"
    echo ""
    echo "Safe to commit."
    exit 0
else
    echo -e "${RED}${#FAILURES[@]} check(s) failed:${NC}"
    for failure in "${FAILURES[@]}"; do
        echo "  - $failure"
    done
    echo ""
    echo "========================================"
    echo -e "${YELLOW}ACTION REQUIRED${NC}"
    echo "========================================"
    echo ""
    echo "Fix these issues before committing:"
    for failure in "${FAILURES[@]}"; do
        case "$failure" in
            *"not formatted"*)
                echo "  - Run 'cargo fmt' to format code"
                ;;
            *"uncommitted"*)
                echo "  - Stage or stash uncommitted changes"
                ;;
            *"Cargo build"*)
                echo "  - Fix Cargo build errors"
                ;;
            *"Cargo tests"*)
                echo "  - Fix failing Cargo tests"
                ;;
            *"Clippy"*)
                echo "  - Fix Clippy warnings"
                ;;
        esac
    done
    echo ""
    echo "Do NOT commit until all checks pass."
    exit 1
fi
