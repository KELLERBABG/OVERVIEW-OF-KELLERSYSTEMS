@echo off
set "ROOT=%CD%"
set "VERIFIER=%ROOT%\target\debug\keller-verify.exe"
set "ENV_HELPER=%ROOT%\target\debug\keller-env.exe"
set "ENV_OUT=%TEMP%\keller-env-%RANDOM%.txt"
"%ENV_HELPER%" --shell plain --manifest-dir "%ROOT%" >"%ENV_OUT%" 2>&1
for /f "tokens=1,* delims==" %%A in ('findstr /b /v /C:"[env]" "%ENV_OUT%"') do call :env_set "%%A" "%%B"
del "%ENV_OUT%" >nul 2>&1
if defined PATH_ADD set "PATH=%PATH_ADD%;%PATH%"
if defined RUSTC (
  for %%I in ("%RUSTC%") do set "TOOLCHAIN_BIN=%%~dpI"
)
if defined TOOLCHAIN_BIN (
  if exist "%VERIFIER%" copy /Y "%VERIFIER%" "%TOOLCHAIN_BIN%keller-verify.exe" >nul
  if exist "%ROOT%\.z3\current\bin\libz3.dll" copy /Y "%ROOT%\.z3\current\bin\libz3.dll" "%TOOLCHAIN_BIN%libz3.dll" >nul
  set "VERIFIER=%TOOLCHAIN_BIN%keller-verify.exe"
)
"%VERIFIER%" keller %*
exit /b %ERRORLEVEL%

:env_set
set "%~1=%~2"
exit /b 0
