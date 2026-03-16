@echo off
REM Build WASM SIMD demo for web browsers (Windows)

echo Building WASM SIMD demo...
echo.

REM Build with wasm-pack (SIMD enabled via .cargo/config.toml)
echo Building with WASM SIMD support (target-feature=+simd128)...
wasm-pack build --target web --release

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Build failed!
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo Build complete!
echo.
echo Generated files:
echo   - pkg\wasm_simd_demo_bg.wasm (WASM binary)
echo   - pkg\wasm_simd_demo.js (JS bindings)
echo.

REM Check for SIMD instructions (requires wabt tools)
where wasm-objdump >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo Checking for SIMD instructions...
    wasm-objdump -d pkg\wasm_simd_demo_bg.wasm 2>nul | findstr /C:"v128" | find /C "v128" > simd_count.tmp
    set /p SIMD_COUNT=<simd_count.tmp
    del simd_count.tmp

    echo Found !SIMD_COUNT! v128 SIMD instructions
    echo.

    if !SIMD_COUNT! GTR 0 (
        echo [32m✓ SIMD compilation successful![0m
        echo.
        echo Sample SIMD instructions:
        wasm-objdump -d pkg\wasm_simd_demo_bg.wasm 2>nul | findstr /C:"v128" | more +0
    ) else (
        echo [33m⚠ Warning: No v128 SIMD instructions found[0m
        echo This may indicate SIMD is not being used
    )
) else (
    echo Note: Install wabt tools to verify SIMD instructions
    echo   Download from https://github.com/WebAssembly/wabt/releases
)

echo.
echo To run the demo:
echo   1. Start a local server: python -m http.server 8000
echo   2. Open http://localhost:8000 in your browser
echo.
pause
