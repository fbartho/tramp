#!/bin/bash
# Test post-hook script
echo "Post-hook: TRAMP_ORIGINAL_BINARY=$TRAMP_ORIGINAL_BINARY"
echo "Post-hook: TRAMP_EXECUTED_BINARY=$TRAMP_EXECUTED_BINARY"
echo "Post-hook: TRAMP_EXIT_CODE=$TRAMP_EXIT_CODE"
exit 0
