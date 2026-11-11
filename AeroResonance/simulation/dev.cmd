@echo off
rem Development wrapper for the EHAR simulation workspace.
rem
rem WHY: if this repo lives under a Windows path containing spaces (e.g.
rem "C:\Users\INTAL Admin\SYSTEMS & CREATIONS\..."), the GNU toolchain's
rem dlltool fails to build windows-sys (a dev-dependency of criterion) with
rem "can't create C:\Users\INTAL: Permission denied". Redirecting the cargo
rem target directory to a space-free path fixes it.
rem
rem Usage:  dev.cmd build --workspace
rem         dev.cmd test --workspace
rem         dev.cmd clippy --workspace --all-targets -- -D warnings
rem         dev.cmd bench --bench run_simulation
rem
rem Override the target directory by setting CARGO_TARGET_DIR yourself.
if "%CARGO_TARGET_DIR%"=="" set "CARGO_TARGET_DIR=C:\ehsim-target"
cargo %*
