@echo off
REM Quick test script to verify the demo works

echo Building MMORPG demo...
cargo build --release
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo Running integration tests...
cargo test --release
if %errorlevel% neq 0 exit /b %errorlevel%

echo.
echo Demo validation complete!
echo.
echo To run the demo manually:
echo   1. Start server: cargo run --release --bin mmorpg-server
echo   2. Start client: cargo run --release --bin mmorpg-client PlayerName
echo.
echo Example:
echo   Terminal 1: cargo run --release --bin mmorpg-server
echo   Terminal 2: cargo run --release --bin mmorpg-client Alice
echo   Terminal 3: cargo run --release --bin mmorpg-client Bob
