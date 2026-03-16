@echo off
REM Verify WASM SIMD compilation (Windows)

echo ==================================
echo WASM SIMD Verification Script
echo ==================================
echo.

REM Check if pkg directory exists
if not exist "pkg\" (
    echo [31mError: pkg\ directory not found[0m
    echo Run 'wasm-pack build --target web --release' first
    pause
    exit /b 1
)

REM Check if WASM binary exists
if not exist "pkg\wasm_simd_demo_bg.wasm" (
    echo [31mError: WASM binary not found[0m
    echo Run 'wasm-pack build --target web --release' first
    pause
    exit /b 1
)

echo [32m✓ Found WASM binary: pkg\wasm_simd_demo_bg.wasm[0m
echo.

REM Check file size
for %%A in ("pkg\wasm_simd_demo_bg.wasm") do set SIZE=%%~zA
echo Binary size: %SIZE% bytes
echo.

REM Check if wasm-tools is installed
where wasm-tools >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [33m⚠ Warning: wasm-tools not found[0m
    echo Install with: cargo install wasm-tools
    echo.
    pause
    exit /b 0
)

echo [32m✓ Found wasm-tools[0m
echo.

REM Convert WASM to WAT
echo Converting WASM to text format...
wasm-tools print pkg\wasm_simd_demo_bg.wasm > pkg\output.wat 2>&1

if %ERRORLEVEL% NEQ 0 (
    echo [31mError: Failed to convert WASM to WAT[0m
    pause
    exit /b 1
)

echo [32m✓ Converted to WAT format[0m
echo.

REM Count SIMD instructions
echo Counting SIMD instructions...
findstr /I /C:"f32x4" /C:"v128" /C:"i32x4" pkg\output.wat > pkg\simd_temp.txt
for /f %%A in ('type pkg\simd_temp.txt ^| find /c /v ""') do set SIMD_COUNT=%%A
del pkg\simd_temp.txt

echo.
echo ==================================
echo SIMD Verification Results
echo ==================================
echo.
echo Total v128 SIMD instructions: %SIMD_COUNT%
echo.

if %SIMD_COUNT% GTR 200 (
    echo [32m✅ SUCCESS: WASM SIMD is working![0m
    echo.
    echo Sample SIMD instructions:
    findstr /I /C:"f32x4" /C:"v128" pkg\output.wat | more +0
    echo.
    echo Expected performance: 2-4x speedup over scalar code
) else (
    echo [31m❌ FAILED: No SIMD instructions found![0m
    echo.
    echo Troubleshooting:
    echo 1. Check .cargo\config.toml has: rustflags = ["-C", "target-feature=+simd128"]
    echo 2. Rebuild: wasm-pack build --target web --release
    echo 3. Verify wide crate supports WASM SIMD
)

echo.
echo ==================================
pause
