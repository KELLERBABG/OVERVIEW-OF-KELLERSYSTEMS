# Reproduce the captured console streams tests/run_tests.sh writes into
# tests/.measured/, on a machine where the bash runner cannot start. The suite
# greps those streams for the lines its later cases assert on, so a Windows
# checkout needs them too; they are not input to anything else any more, since
# index.html is hand-written and nothing reads the manifest for a page.
#
# The solver budget is the one difference from the suite: `undecided` hands Z3 a
# nonlinear obligation that does not return on this machine without a budget, and
# a bounded wait yields the same `unknown` verdict.
$ErrorActionPreference = 'Continue'
$repo = 'C:\Users\INTAL Admin\SYSTEMS & CREATIONS\Formal Verification Platform'
$tc = Join-Path $HOME '.rustup\toolchains\nightly-2026-07-24-x86_64-pc-windows-gnu'
$env:PATH = "$tc\lib\rustlib\x86_64-pc-windows-gnu\lib;$repo\.z3\current\bin;$env:PATH"
Remove-Item Env:\RUSTUP_TOOLCHAIN -ErrorAction SilentlyContinue

$cases = @(
  @{c='safe_code';      e=0; w=2},
  @{c='overflow_fail';  e=1},
  @{c='divzero_fail';   e=1},
  @{c='branch_merge';   e=1},
  @{c='guarded';        e=1},
  @{c='workspace_multi';e=1},
  @{c='loop_fail';      e=1},
  @{c='nested_loop';    e=1},
  @{c='shift_amount';   e=1},
  @{c='stable_pin';     e=1},
  @{c='fold_fail';      e=1},
  @{c='broken_code';    e=3},
  @{c='undecided';      e=0; w=1; timeout=10000},
  @{c='unnamed_write';  e=0; w=1},
  @{c='statement_writes'; e=0; w=4}
)

$measured = Join-Path $repo 'tests\.measured'
Remove-Item $measured -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $measured -Force | Out-Null

$manifest = @()
$bad = 0
foreach ($case in $cases) {
  $dir = Join-Path $repo "tests\$($case.c)"
  $args = @('keller', 'verify')
  if ($case.timeout) { $args += @('--solver-timeout', $case.timeout) }
  Push-Location $dir
  # Redirected through a stream rather than the pipeline: PowerShell wraps `n`
  # characters it captures and inserts a newline, which split a long `[WARN]` line
  # in two and made the page generator count one warning where the run printed two.
  $log = Join-Path $measured "$($case.c).log"
  $utf8 = New-Object System.Text.UTF8Encoding($false)
  $writer = New-Object System.IO.StreamWriter($log, $false, $utf8)
  $writer.AutoFlush = $true
  & "$repo\target\debug\keller-verify.exe" @args 2>&1 | ForEach-Object { $writer.WriteLine($_) }
  $code = $LASTEXITCODE
  $writer.Close()
  Pop-Location
  $line = if ($case.w) { "$($case.c)`t$($case.e)`t$($case.w)" } else { "$($case.c)`t$($case.e)" }
  $manifest += $line
  $mark = if ($code -eq $case.e) { 'ok  ' } else { 'BAD '; }
  if ($code -ne $case.e) { $bad++ }
  "$mark$($case.c): exit=$code expected=$($case.e)"
}
Set-Content -Path (Join-Path $measured 'manifest.tsv') -Value $manifest -Encoding ASCII
if ($bad -gt 0) { Write-Output "$bad case(s) did not exit as the manifest expects"; exit 1 }
Write-Output 'MEASURED RUN OK'
