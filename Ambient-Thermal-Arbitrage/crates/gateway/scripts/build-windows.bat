@echo off
REM Build/test the ATA gateway on Windows.
REM Requires MSYS2 mingw-w64 gcc on PATH (ring/aws-lc crypto needs a C compiler):
REM   pacman -S --noconfirm mingw-w64-x86_64-gcc
REM Uses a space-free target dir because GNU dlltool breaks on paths with spaces.
REM Usage: scripts\build-windows.bat [test|build|run]
setlocal

set "CARGO_TARGET_DIR=C:\ata-target"
set "MODE=%~1"
if "%MODE%"=="" set "MODE=build"

where gcc >nul 2>&1
if errorlevel 1 (
  echo [ERROR] gcc not found on PATH. Install MSYS2 and run:
  echo   pacman -S --noconfirm mingw-w64-x86_64-gcc
  echo then add C:\msys64\mingw64\bin to PATH.
  exit /b 1
)

cd /d "%~dp0.."

if /i "%MODE%"=="test" (
  cargo +nightly-x86_64-pc-windows-gnu test --quiet
) else if /i "%MODE%"=="run" (
  cargo +nightly-x86_64-pc-windows-gnu run --release
) else (
  cargo +nightly-x86_64-pc-windows-gnu build --release
)

exit /b %ERRORLEVEL%
