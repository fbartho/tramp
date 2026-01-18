#!/bin/bash
# Test pre-hook script
echo "Pre-hook: TRAMP_ORIGINAL_BINARY=$TRAMP_ORIGINAL_BINARY"
echo "Pre-hook: TRAMP_ORIGINAL_ARGS=$TRAMP_ORIGINAL_ARGS"
echo "Pre-hook: TRAMP_CWD=$TRAMP_CWD"
echo "Pre-hook: TRAMP_HOOK_TYPE=$TRAMP_HOOK_TYPE"
exit 0
