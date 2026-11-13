param([switch]$ValidateOnly)

$ErrorActionPreference = 'Stop'

# Compute root as parent of script's directory
$Root = Split-Path -Parent $PSScriptRoot
$verifier = Join-Path $Root 'keller-verify.cmd'

# Define the eight cases in order
$cases = @(
    @{ Lang='ts';     Dir='typescript_cases' },
    @{ Lang='sql';    Dir='sql_cases' },
    @{ Lang='c';      Dir='c_cases' },
    @{ Lang='cpp';    Dir='cpp_cases' },
    @{ Lang='asm';    Dir='asm_cases' },
    @{ Lang='go';     Dir='go_cases' },
    @{ Lang='java';   Dir='java_cases' },
    @{ Lang='csharp'; Dir='csharp_cases' }
)

# ValidateOnly mode: verify verifier exists, print planned commands, exit 0
if ($ValidateOnly) {
    if (-not (Test-Path $verifier)) {
        Write-Output "FAIL: verifier not found at $verifier"
        exit 1
    }
    foreach ($case in $cases) {
        $fixture = Join-Path $Root ("tests/" + $case.Dir)
        $outJson = Join-Path $env:TEMP ("keller_audit_" + $case.Lang + ".json")
        Write-Output "PLANNED: & `"$verifier`" audit --lang $($case.Lang) `"$fixture`" --out `"$outJson`""
    }
    exit 0
}

# Check verifier exists
if (-not (Test-Path $verifier)) {
    Write-Output "FAIL: verifier not found at $verifier. Build with: cargo build --features all-langs"
    exit 1
}

# Create temporary output directory
$tmpDir = Join-Path $env:TEMP ("keller_lang_audit_" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

$failed = $false
$passedCount = 0

try {
    # Loop over the eight language cases
    foreach ($case in $cases) {
        $lang = $case.Lang
        $dir = $case.Dir
        $fixture = Join-Path $Root ("tests/" + $dir)
        $outJson = Join-Path $tmpDir ($lang + "_report.json")

        # Run the verifier audit
        & $verifier audit --lang $lang "$fixture" --out $outJson
        $exitCode = $LASTEXITCODE

        if ($exitCode -ne 0) {
            Write-Output "FAIL $lang : verifier exited with code $exitCode"
            $failed = $true
            continue
        }

        # Read and validate JSON report
        $jsonContent = Get-Content -Raw -Path $outJson
        $report = $jsonContent | ConvertFrom-Json

        if ($null -eq $report) {
            Write-Output "FAIL $lang : could not parse JSON report"
            $failed = $true
            continue
        }

        # Assert files_scanned > 0
        if ($report.files_scanned -le 0) {
            Write-Output "FAIL $lang : files_scanned is $($report.files_scanned), expected > 0"
            $failed = $true
            continue
        }

        # Assert files_unparsed == 0
        if ($report.files_unparsed -ne 0) {
            Write-Output "FAIL $lang : files_unparsed is $($report.files_unparsed), expected 0"
            $failed = $true
            continue
        }

        # Assert findings.Count > 0
        if ($null -eq $report.findings -or $report.findings.Count -le 0) {
            Write-Output "FAIL $lang : no findings in report, expected at least 1"
            $failed = $true
            continue
        }

        # Assert obligations_proved_safe <= obligations_total
        if ($report.obligations_proved_safe -gt $report.obligations_total) {
            Write-Output "FAIL $lang : obligations_proved_safe ($($report.obligations_proved_safe)) > obligations_total ($($report.obligations_total))"
            $failed = $true
            continue
        }

        # All checks passed for this case
        $passedCount++
        Write-Output "PASS $lang : $($report.files_scanned) files, $($report.findings.Count) findings"
    }

    # Negative check 1: bogus language should exit 2
    & $verifier audit --lang bogus tests/c_cases
    $negExit1 = $LASTEXITCODE
    if ($negExit1 -ne 2) {
        Write-Output "FAIL negative : expected exit code 2 for bogus lang, got $negExit1"
        $failed = $true
    }

    # Negative check 2: non-existent fixture should exit 2
    & $verifier audit --lang c tests/does_not_exist
    $negExit2 = $LASTEXITCODE
    if ($negExit2 -ne 2) {
        Write-Output "FAIL negative : expected exit code 2 for missing fixture, got $negExit2"
        $failed = $true
    }

    # Negative check 3: --out inside audited tree should be refused with exit 2
    $badOut = Join-Path $Root 'tests/typescript_cases/should_not_exist.json'
    & $verifier audit --lang ts tests/typescript_cases --out $badOut
    $negExit3 = $LASTEXITCODE
    if ($negExit3 -ne 2) {
        Write-Output "FAIL negative : expected exit code 2 for out-inside-tree, got $negExit3"
        $failed = $true
    } else {
        # Verify no file was created at the refused path
        if (Test-Path $badOut) {
            Write-Output "FAIL negative : file was created at $badOut despite refusal"
            $failed = $true
        }
    }
}
finally {
    # Clean up temporary directory
    if (Test-Path $tmpDir) {
        Remove-Item -Recurse -Force -Path $tmpDir
    }
}

# Print summary
$totalCases = $cases.Count
Write-Output "SUMMARY: $passedCount/$totalCases passed"

if ($failed) {
    exit 1
} else {
    exit 0
}
