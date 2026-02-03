@echo off
echo Building client...
cargo build --bin client 2>&1
echo.
echo Exit code: %ERRORLEVEL%
echo.
if %ERRORLEVEL% EQU 0 (
    echo Build succeeded!
) else (
    echo Build failed!
)
