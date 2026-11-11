#Requires -Version 5.1
<#
.SYNOPSIS
    Stage a relocatable Keller-Verify release archive for Windows.

.DESCRIPTION
    This is the PowerShell counterpart of scripts/package-release.sh. The bundle
    contains the executables, the Z3 DLL they linked against, and a `keller.cmd`
    launcher that resolves the pinned toolchain through rustup. It is not a
    statically linked binary: the verifier reads MIR through `rustc_private`, so
    the pinned nightly must be installed on the target machine.

    Run after `cargo build --release --locked`.

.PARAMETER OutputDirectory
    Where to write the archive. Defaults to <repo>\dist.

.EXAMPLE
    pwsh scripts/package-release.ps1
#>
[CmdletBinding()]
param(
    [string]$OutputDirectory = ''
)

$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $repo 'dist'
}

function Fail([string]$Message) {
    Write-Error "[keller-package] $Message"
    exit 1
}

function Read-FirstMatch([string]$Path, [string]$Pattern) {
    $match = Select-String -Path $Path -Pattern $Pattern | Select-Object -First 1
    if ($null -eq $match) { return '' }
    return $match.Matches[0].Groups[1].Value
}

$channel = Read-FirstMatch (Join-Path $repo 'rust-toolchain.toml') '^channel = "([^"]+)"'
$version = Read-FirstMatch (Join-Path $repo 'Cargo.toml') '^version = "([^"]+)"'
if ([string]::IsNullOrWhiteSpace($channel)) { Fail 'could not read the pinned channel from rust-toolchain.toml' }
if ([string]::IsNullOrWhiteSpace($version)) { Fail 'could not read the package version from Cargo.toml' }

# Ask rustup which toolchain cargo actually used, in the repository, rather than
# resolving the bare channel here. On a machine with both host builds installed,
# `--toolchain nightly-...` can pick a different host than the active override;
# that packaged a GNU binary as an MSVC bundle on the development checkout, with
# the launcher then looking for the wrong driver libraries.
Push-Location $repo
try {
    $active = (& rustup show active-toolchain 2>$null | Select-Object -First 1)
} finally {
    Pop-Location
}
if ([string]::IsNullOrWhiteSpace($active)) {
    Fail "rustup reports no active toolchain; run 'rustup default $channel' or add a rust-toolchain.toml override"
}
$toolchain = ($active -split '\s+')[0]
if (-not $toolchain.StartsWith($channel)) {
    Fail "the active toolchain '$toolchain' is not the pinned channel '$channel'; packaging would bundle a different compiler"
}

$rustc = (& rustup which --toolchain $toolchain rustc 2>$null | Select-Object -First 1)
if ([string]::IsNullOrWhiteSpace($rustc)) { Fail "rustup could not resolve the active toolchain '$toolchain'" }
$rustc = $rustc.Trim()

$hostLine = (& $rustc -vV | Where-Object { $_ -like 'host: *' } | Select-Object -First 1)
if ([string]::IsNullOrWhiteSpace($hostLine)) { Fail 'could not determine the host triple from rustc -vV' }
$target = $hostLine.Substring('host: '.Length).Trim()
$release = ((& $rustc -vV) | Where-Object { $_ -like 'release: *' } | Select-Object -First 1).Substring('release: '.Length).Trim()
$commit = ((& $rustc -vV) | Where-Object { $_ -like 'commit-hash: *' } | Select-Object -First 1).Substring('commit-hash: '.Length).Trim()

$z3Home = $env:Z3_HOME
if ([string]::IsNullOrWhiteSpace($z3Home) -and (Test-Path (Join-Path $repo '.z3\current'))) {
    $z3Home = Join-Path $repo '.z3\current'
}
if ([string]::IsNullOrWhiteSpace($z3Home) -or -not (Test-Path $z3Home)) {
    Fail 'Z3_HOME is not set and no .z3\current exists'
}
$z3Lib = Join-Path $z3Home 'bin\libz3.dll'
if (-not (Test-Path $z3Lib)) { Fail "Z3_HOME has no bin\libz3.dll: $z3Home" }

$targetDir = $env:KELLER_CARGO_TARGET_DIR
if ([string]::IsNullOrWhiteSpace($targetDir)) { $targetDir = Join-Path $repo 'target' }
$binary = $null
foreach ($candidate in @((Join-Path $targetDir "$target\release\keller-verify.exe"), (Join-Path $targetDir 'release\keller-verify.exe'))) {
    if (Test-Path $candidate) { $binary = $candidate; break }
}
if ($null -eq $binary) {
    Fail "no release build at $targetDir\$target\release\keller-verify.exe or $targetDir\release\keller-verify.exe; run 'cargo build --release --locked' first"
}

$name = "keller-verify-$version-$target"
$stage = Join-Path $OutputDirectory $name
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
foreach ($dir in @('bin', 'z3\bin', 'z3\include', 'docs')) {
    New-Item -ItemType Directory -Path (Join-Path $stage $dir) -Force | Out-Null
}

foreach ($exe in @('keller-verify', 'keller-env', 'cargo-keller')) {
    $source = Join-Path (Split-Path -Parent $binary) "$exe.exe"
    if (-not (Test-Path $source)) { Fail "release build did not produce $exe.exe beside keller-verify.exe" }
    Copy-Item $source (Join-Path $stage "bin\$exe.exe")
}

Copy-Item (Join-Path $z3Home 'bin\*') (Join-Path $stage 'z3\bin') -Recurse -Force
Copy-Item (Join-Path $z3Home 'include\z3.h') (Join-Path $stage 'z3\include\z3.h') -Force
$z3Version = (& (Join-Path $z3Home 'bin\z3.exe') --version 2>$null | Select-Object -First 1)

$launcher = @'
@echo off
setlocal
set "HERE=%~dp0"
for %%I in ("%HERE%..") do set "ROOT=%%~fI"
set "TOOLCHAIN=@TOOLCHAIN@"
for /f "delims=" %%R in ('rustup which --toolchain %TOOLCHAIN% rustc 2^>nul') do set "RUSTC=%%R"
if not defined RUSTC (
  echo keller: the pinned Rust toolchain %TOOLCHAIN% is not installed. 1>&2
  echo Install it with: rustup toolchain install %TOOLCHAIN% --component rustc-dev --component llvm-tools-preview 1>&2
  exit /b 3
)
rem The host is embedded at packaging time rather than parsed from `rustc -vV`
rem here: a `for /f` command string loses the quotes around a path with a space,
rem and the target this archive was built for is already known. It also keeps the
rem launcher from silently selecting a different host's driver directory.
set "HOST=@HOST@"
for %%I in ("%RUSTC%") do set "RUSTLIB=%%~dpI..\lib"
set "DRIVER=%RUSTLIB%\rustlib\%HOST%\lib"
if not exist "%DRIVER%" (
  echo keller: pinned rustc driver libraries missing: %DRIVER% 1>&2
  exit /b 3
)
set "RUSTUP_TOOLCHAIN=%TOOLCHAIN%"
set "Z3_HOME=%ROOT%\z3"
set "PATH=%HERE%;%ROOT%\z3\bin;%DRIVER%;%PATH%"
"%HERE%keller-verify.exe" %*
exit /b %ERRORLEVEL%
'@
$launcher = $launcher.Replace('@TOOLCHAIN@', $toolchain).Replace('@HOST@', $target)
Set-Content -Path (Join-Path $stage 'bin\keller.cmd') -Value $launcher -Encoding ASCII

Copy-Item (Join-Path $repo 'README.md') $stage
Copy-Item (Join-Path $repo 'LICENSE-MIT') $stage
Copy-Item (Join-Path $repo 'LICENSE-APACHE') $stage
Copy-Item (Join-Path $repo 'docs\RELEASE.md') (Join-Path $stage 'docs\RELEASE.md')

$z3Hash = (Get-FileHash -Path $z3Lib -Algorithm SHA256).Hash.ToLowerInvariant()
$manifest = [ordered]@{
    tool = 'keller-verify'
    version = $version
    target = $target
    rustc = [ordered]@{ release = $release; commit = $commit; host = $target }
    toolchain = $toolchain
    z3_version = $z3Version
    z3_sha256 = $z3Hash
    runtime = 'requires the pinned Rust toolchain (rustc-dev, llvm-tools-preview); Z3 is bundled'
}
$manifest | ConvertTo-Json -Depth 4 | Set-Content -Path (Join-Path $stage 'manifest.json') -Encoding UTF8

New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$archive = Join-Path $OutputDirectory "$name.zip"
if (Test-Path $archive) { Remove-Item -Force $archive }
Compress-Archive -Path $stage -DestinationPath $archive -CompressionLevel Optimal
$archiveHash = (Get-FileHash -Path $archive -Algorithm SHA256).Hash.ToLowerInvariant()
[System.IO.File]::WriteAllText("$archive.sha256", "$archiveHash  $name.zip`n")

Write-Output "[keller-package] $archive"
Write-Output "[keller-package] $archive.sha256"
