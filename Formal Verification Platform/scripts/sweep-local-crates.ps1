# Run keller verify over local Rust crates and record what the analysis did on
# code it was not written for. `keller verify` pins nothing for the target
# project, so a crate whose rust-toolchain.toml says "stable" fails cargo's -Z
# flag; pinning RUSTUP_TOOLCHAIN to the toolchain the verifier links against is
# the workaround this script exercises.
param(
  [string[]]$Only,
  [string]$OutFile,
  # One run per crate, capped at this many seconds (0 = no cap, the default).
  # The cap is handed to the tool itself (`--run-timeout`), so a crate that stops
  # making progress is stopped by the run and recorded as a stopped run rather
  # than by this script; the wait below is the backstop for a run that never
  # reaches that logic, which is why it is longer than the cap. A crate that does
  # not finish is reported as TIMEOUT and leaves no report.json — a partial run is
  # not a measurement, so it is recorded as not measured rather than as a crate
  # with no findings. The measured rates in docs/FALSE-POSITIVE-RATE.md were taken
  # this way: seven of sixteen crates completed — five with findings, two with
  # none — and eight ran past their cap and were killed.
  [int]$TimeoutSeconds = 0
)
$ErrorActionPreference = 'Continue'
$root = 'C:\Users\INTAL Admin\SYSTEMS & CREATIONS'
$repo = 'C:\Users\INTAL Admin\SYSTEMS & CREATIONS\Formal Verification Platform'
$tc = Join-Path $HOME '.rustup\toolchains\nightly-2026-07-24-x86_64-pc-windows-gnu'
$env:PATH = "$tc\lib\rustlib\x86_64-pc-windows-gnu\lib;$repo\.z3\current\bin;$env:PATH"
$env:RUSTUP_TOOLCHAIN = 'nightly-2026-07-24'

$targets = @(
  @{n='city-types';   p='City OS\crates\city-types'},
  @{n='city-crypto';  p='City OS\crates\city-crypto'},
  @{n='city-caps';    p='City OS\crates\city-caps'},
  @{n='city-brp';     p='City OS\crates\city-brp'},
  @{n='city-dag';     p='City OS\crates\city-dag'},
  @{n='op-math';      p='OP-Roboter\crates\op-math'},
  @{n='op-kinematics';p='OP-Roboter\crates\op-kinematics'},
  @{n='op-gate';      p='OP-Roboter\crates\op-gate'},
  @{n='sca-types';    p='Algorithmic Supply Chain Arbitrage\crates\sca-types'},
  @{n='sca-matching'; p='Algorithmic Supply Chain Arbitrage\crates\sca-matching'},
  @{n='sca-security'; p='Algorithmic Supply Chain Arbitrage\crates\sca-security'},
  @{n='smpc-zk-validation';      p='SMPC Dark Pool\crates\zk-validation'},
  @{n='smpc-homomorphic-compare';p='SMPC Dark Pool\crates\homomorphic-compare'},
  @{n='smpc-threshold-crypto';   p='SMPC Dark Pool\crates\threshold-crypto'},
  @{n='smpc-engine';             p='SMPC Dark Pool\crates\smpc-engine'},
  @{n='smpc-commitments';        p='SMPC Dark Pool\crates\commitments'},
  @{n='transit-state-machine';   p='Transit OS\crates\state-machine'},
  @{n='transit-path-constraints';p='Transit OS\crates\path-constraints'},
  @{n='transit-smt-solver';      p='Transit OS\crates\smt-solver'},
  @{n='zk-genome-core';          p='Zero-Knowledge Genome\core\genome-core'},
  @{n='zk-core';                 p='Zero-Knowledge Genome\core\zk-core'},
  @{n='pod-physics-logic';       p='Autnomous Proof-of-Damage Layer\crates\physics-logic'},
  @{n='pod-sensor-integrity';    p='Autnomous Proof-of-Damage Layer\crates\sensor-integrity'},
  @{n='dfip-core';               p='Decoupled Fluid IP Network\core'},
  @{n='sybil-core';              p='Sybil-Resistentes Proof-Of-Personhood\crates\core'}
)
if ($Only) { $targets = $targets | Where-Object { $Only -contains $_.n } }

function Measure-Report($dir) {
  $report = Join-Path $dir 'report.json'
  if (-not (Test-Path $report)) { return $null }
  $findings = [object[]](ConvertFrom-Json -InputObject (Get-Content $report -Raw))
  $by = @{}
  foreach ($g in ($findings | Group-Object status)) { $by[$g.Name] = $g.Count }
  $kinds = ($findings | Where-Object { $_.kind } | Group-Object kind |
            ForEach-Object { "$($_.Name)=$($_.Count)" }) -join ','
  $loc = @($findings | Where-Object { $_.file } | ForEach-Object { $_.file } | Select-Object -Unique).Count
  $witnesses = @($findings | Where-Object { $_.status -eq 'FAIL' -and $_.witness } |
                 ForEach-Object { ($_.witness | ForEach-Object { $_.value }) -join ' ' })
  $extreme = @($witnesses | Where-Object { $_ -match '184467440737095516|9223372036854775807|340282366920938463463374607431768211' })
  return [ordered]@{
    pass = [int]$by['PASS']; fail = [int]$by['FAIL']; warn = [int]$by['WARN']
    fail_extreme_witness = $extreme.Count
    fail_kinds = $kinds
    files_with_findings = $loc
  }
}

$results = @()
foreach ($t in $targets) {
  $dir = Join-Path $root $t.p
  if (-not (Test-Path $dir)) { Write-Output "MISSING $($t.n)"; continue }
  Write-Output "===== $($t.n) ====="
  Remove-Item (Join-Path $dir 'report.json') -Force -ErrorAction SilentlyContinue
  $sw = [Diagnostics.Stopwatch]::StartNew()
  $exe = "$repo\target\debug\keller-verify.exe"
  $log = Join-Path $dir 'keller-sweep.log'
  $timedOut = $false
  if ($TimeoutSeconds -gt 0) {
    # Start-Process rather than `&`, because a cap needs a handle to the process:
    # `timeout` is a POSIX tool and this is the Windows-side script. Stdout and
    # stderr go to two files (Start-Process refuses one) and are concatenated
    # afterwards, so the log still reads like one stream.
    $err = Join-Path $dir 'keller-sweep.err.log'
    $proc = Start-Process -FilePath $exe -ArgumentList 'keller', 'verify', '--quiet', '--run-timeout', "$TimeoutSeconds" `
      -WorkingDirectory $dir -NoNewWindow -PassThru `
      -RedirectStandardOutput $log -RedirectStandardError $err
    try {
      # Longer than the cap the tool was given: the tool stopping itself is the
      # normal path, and reaching this deadline means it could not.
      $proc | Wait-Process -Timeout ($TimeoutSeconds + 60) -ErrorAction Stop
      $code = $proc.ExitCode
      if (Select-String -Path $log -Pattern 'Stopped at the --run-timeout cap' -Quiet -ErrorAction SilentlyContinue) {
        $timedOut = $true
      }
    } catch {
      $timedOut = $true
      $code = -9
      # The tree, not the process. `Stop-Process` here killed the verifier and left
      # what it had started: the workspace wrappers and their cargo are the
      # verifier's *children*, so a sweep attempt used to leak one running analysis
      # per cap. Measured: a `keller-verify` -> `cargo` -> `cargo` ->
      # `workspace-wrapper` tree from an earlier attempt of this script was still
      # alive six hours later. `taskkill /T` walks the parent chain, and the tool
      # does the same thing for its own `--run-timeout`.
      & taskkill /T /F /PID $proc.Id 2>&1 | Out-Null
      if ($LASTEXITCODE -ne 0) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
      }
    }
    # A killed process can still hold its redirect for a moment, so the streams
    # are joined with a pause and a fallback: if the log cannot be appended to,
    # the stderr sidecar stays exactly where it is rather than being dropped.
    Start-Sleep -Milliseconds 500
    $joined = $true
    try {
      Get-Content $err -ErrorAction Stop | Out-File -FilePath $log -Append -Encoding utf8
    } catch {
      $joined = $false
    }
    if ($joined) { Remove-Item $err -Force -ErrorAction SilentlyContinue }
    if ($timedOut) { Write-Output "TIMEOUT $($t.n) after $TimeoutSeconds s (not measured)" }  } else {
    Push-Location $dir
    & $exe keller verify --quiet 2>&1 | Out-File -FilePath $log -Encoding utf8
    $code = $LASTEXITCODE
    Pop-Location
  }
  $sw.Stop()
  $rec = [ordered]@{ crate = $t.n; exit = $code; seconds = [math]::Round($sw.Elapsed.TotalSeconds,1) }
  $m = Measure-Report $dir
  if ($m) { foreach ($k in $m.Keys) { $rec[$k] = $m[$k] } } else { $rec.report_json = 'none' }
  # A capped run that was killed must not read as "a crate with nothing to report".
  if ($timedOut) { $rec['timed_out'] = $true }
  $results += [pscustomobject]$rec
  $results[-1] | Format-List
}
$dest = if ($OutFile) { $OutFile } else { Join-Path $repo 'sweep-results.csv' }
$results | Export-Csv -Path $dest -NoTypeInformation -Encoding utf8
Write-Output "SWEEP DONE"
$results | Format-Table -AutoSize
