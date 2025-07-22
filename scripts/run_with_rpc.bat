@echo off
:: Simple batch script to run the mempool monitor with a custom RPC endpoint on Windows
:: Usage: run_with_rpc.bat <RPC_URL> <WS_URL>

setlocal enabledelayedexpansion

:: Check if RPC and WS URLs are provided
if "%~1"=="" (
    echo Error: RPC URL is required
    echo Usage: %~nx0 ^<RPC_URL^> ^<WS_URL^>
    echo Example: %~nx0 https://api.mainnet-beta.solana.com wss://api.mainnet-beta.solana.com
    exit /b 1
)

if "%~2"=="" (
    echo Error: WebSocket URL is required
    echo Usage: %~nx0 ^<RPC_URL^> ^<WS_URL^>
    echo Example: %~nx0 https://api.mainnet-beta.solana.com wss://api.mainnet-beta.solana.com
    exit /b 1
)

set RPC_URL=%~1
set WS_URL=%~2

:: Set default log level if not set
if "%RUST_LOG%"=="" (
    set RUST_LOG=info
)

echo Starting Solana Mempool Sniper with:
echo RPC URL: %RPC_URL%
echo WebSocket URL: %WS_URL%
echo Log level: %RUST_LOG%
echo.

:: Run the example with the custom RPC endpoint
set RPC_URL=%RPC_URL%
set WS_URL=%WS_URL%
set RUST_LOG=%RUST_LOG%

cargo run --release --example custom_config

endlocal
