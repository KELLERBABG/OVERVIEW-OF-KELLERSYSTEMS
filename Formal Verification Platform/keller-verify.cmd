@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
if "%SCRIPT_DIR:~-1%"=="\" set "SCRIPT_DIR=%SCRIPT_DIR:~0,-1%"

set "KELLER_BIN="
if exist "%SCRIPT_DIR%\target\release\keller-verify.exe" (
    set "KELLER_BIN=%SCRIPT_DIR%\target\release\keller-verify.exe"
) else if exist "%SCRIPT_DIR%\target\debug\keller-verify.exe" (
    set "KELLER_BIN=%SCRIPT_DIR%\target\debug\keller-verify.exe"
) else if exist "%SCRIPT_DIR%\keller-verify.exe" (
    set "KELLER_BIN=%SCRIPT_DIR%\keller-verify.exe"
)

set "ENV_HELPER="
if exist "%SCRIPT_DIR%\target\release\keller-env.exe" (
    set "ENV_HELPER=%SCRIPT_DIR%\target\release\keller-env.exe"
) else if exist "%SCRIPT_DIR%\target\debug\keller-env.exe" (
    set "ENV_HELPER=%SCRIPT_DIR%\target\debug\keller-env.exe"
) else if exist "%SCRIPT_DIR%\keller-env.exe" (
    set "ENV_HELPER=%SCRIPT_DIR%\keller-env.exe"
)

set "ENV_TMP=%TEMP%\keller-env-%RANDOM%.txt"
set "ENV_PATH_ADD="
set "RUSTC_EXE="
"%ENV_HELPER%" --shell plain --manifest-dir "%SCRIPT_DIR%" >"%ENV_TMP%" 2>nul
if exist "%ENV_TMP%" (
    for /f "usebackq tokens=1,* delims==" %%A in ("%ENV_TMP%") do (
        if "%%A"=="PATH_ADD" set "ENV_PATH_ADD=%%B"
        if "%%A"=="RUSTC" set "RUSTC_EXE=%%B"
    )
    del "%ENV_TMP%" >nul 2>&1
)

if defined ENV_PATH_ADD set "PATH=%ENV_PATH_ADD%;%PATH%"
set "PATH=%SCRIPT_DIR%\.z3\current\bin;C:\z3\bin;%PATH%"

set "TOOLCHAIN_BIN="
if defined RUSTC_EXE (
    for %%F in ("%RUSTC_EXE%") do set "TOOLCHAIN_BIN=%%~dpF"
)
if defined TOOLCHAIN_BIN (
    if exist "%TOOLCHAIN_BIN%rustc.exe" (
        if exist "%SCRIPT_DIR%\.z3\current\bin\libz3.dll" if not exist "%TOOLCHAIN_BIN%libz3.dll" (
            copy /y "%SCRIPT_DIR%\.z3\current\bin\libz3.dll" "%TOOLCHAIN_BIN%libz3.dll" >nul 2>&1
        )
        if exist "%KELLER_BIN%" (
            copy /y "%KELLER_BIN%" "%TOOLCHAIN_BIN%keller-verify.exe" >nul 2>&1
            set "KELLER_BIN=%TOOLCHAIN_BIN%keller-verify.exe"
        )
    )
)

if exist "C:\msys64\mingw64\bin" set "PATH=C:\msys64\mingw64\bin;%PATH%"

if "%KELLER_BIN%"=="" (
    echo [keller-verify] error: keller-verify.exe not found. Build it with `cargo build --bin keller-verify`. 1>&2
    exit /b 1
)

"%KELLER_BIN%" %*
exit /b %ERRORLEVEL%
