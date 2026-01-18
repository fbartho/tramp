#!/bin/bash
# Test intercept hook script - completely replaces command execution
echo "Intercept hook executed!"
echo "TRAMP_ORIGINAL_BINARY=$TRAMP_ORIGINAL_BINARY"
echo "TRAMP_ORIGINAL_ARGS=$TRAMP_ORIGINAL_ARGS"
echo "TRAMP_CWD=$TRAMP_CWD"
echo "TRAMP_HOOK_TYPE=$TRAMP_HOOK_TYPE"
echo "TRAMP_EXECUTED_BINARY=$TRAMP_EXECUTED_BINARY"
echo "TRAMP_EXECUTED_ARGS=$TRAMP_EXECUTED_ARGS"
# Exit with success - this becomes tramp's exit code
exit 0
