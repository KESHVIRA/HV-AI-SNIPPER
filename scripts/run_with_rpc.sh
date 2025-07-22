#!/bin/bash

# Simple script to run the mempool monitor with a custom RPC endpoint
# Usage: ./scripts/run_with_rpc.sh <RPC_URL> <WS_URL>

set -e

# Check if RPC and WS URLs are provided
if [ -z "$1" ] || [ -z "$2" ]; then
    echo "Error: RPC and WebSocket URLs are required"
    echo "Usage: $0 <RPC_URL> <WS_URL>"
    echo "Example: $0 https://api.mainnet-beta.solana.com wss://api.mainnet-beta.solana.com"
    exit 1
fi

RPC_URL="$1"
WS_URL="$2"

# Export environment variables
export RPC_URL
# shellcheck disable=SC2155
export WS_URL

# Set default log level if not set
if [ -z "${RUST_LOG}" ]; then
    export RUST_LOG=info
fi

echo "Starting Solana Mempool Sniper with:"
echo "RPC URL: $RPC_URL"
echo "WebSocket URL: $WS_URL"
echo "Log level: $RUST_LOG"
echo""

# Run the example with the custom RPC endpoint
cargo run --release --example custom_config
