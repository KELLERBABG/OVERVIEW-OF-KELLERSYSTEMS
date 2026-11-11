# Smoke test: start node-daemon, hit /api/health, verify, stop.
$ErrorActionPreference = "Stop"

$env:NODE_ID = "00000000-0000-0000-0000-000000000000"
$env:LISTEN_ADDR = "127.0.0.1:19000"
$env:API_ADDR = "127.0.0.1:18080"
$env:PEERS = "00000000-0000-0000-0000-000000000001@127.0.0.1:19001"

$bin = Join-Path $PSScriptRoot "..\target\release\node-daemon.exe"
if (-not (Test-Path $bin)) {
    Write-Output "SMOKE TEST FAILED: binary not found at $bin (run `cargo build --release --bin node-daemon` first)"
    exit 1
}

$p = Start-Process -FilePath $bin `
    -PassThru -NoNewWindow `
    -RedirectStandardOutput "daemon_out.log" `
    -RedirectStandardError "daemon_err.log"

try {
    Start-Sleep -Seconds 20
    $resp = Invoke-RestMethod -Uri "http://127.0.0.1:18080/api/health" -TimeoutSec 5
    Write-Output ("HEALTH: " + ($resp | ConvertTo-Json -Compress))
    if ($resp.status -ne "ok") {
        Write-Output "SMOKE TEST FAILED: unhealthy status"
        exit 1
    }

    # Verify the trading console UI is served at the root route
    $ui = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:18080/" -TimeoutSec 5
    Write-Output ("UI STATUS: " + $ui.StatusCode)
    Write-Output ("UI CONTAINS TITLE: " + $ui.Content.Contains("SMPC Dark Pool"))
    if ($ui.StatusCode -ne 200 -or -not $ui.Content.Contains("SMPC Dark Pool")) {
        Write-Output "SMOKE TEST FAILED: trading console not served"
        exit 1
    }

    # Verify field-validation at the API barrier: zero price must be rejected
    try {
        $null = Invoke-WebRequest -UseBasicParsing -Method Post -Uri "http://127.0.0.1:18080/api/orders" -ContentType "application/json" -Body '{"party_id":1,"side":"bid","price":"0","quantity":"10"}' -TimeoutSec 5
        Write-Output "SMOKE TEST FAILED: zero price was accepted"
        exit 1
    } catch {
        if ($_.Exception.Response.StatusCode.value__ -eq 400) {
            Write-Output "VALIDATION: zero price rejected with 400"
        } else {
            Write-Output ("VALIDATION UNEXPECTED: " + $_.Exception.Response.StatusCode.value__)
            exit 1
        }
    }

    # Verify bucketed book depth in the public snapshot
    $book = Invoke-RestMethod -Uri "http://127.0.0.1:18080/api/book" -TimeoutSec 5
    $validBuckets = @(0, 2, 4, 8, 16, 32, 64, 128, 256)
    if ($validBuckets -notcontains $book.bid_depth -or $validBuckets -notcontains $book.ask_depth) {
        Write-Output "SMOKE TEST FAILED: book depths are not bucketed (bid=$($book.bid_depth), ask=$($book.ask_depth))"
        exit 1
    }
    Write-Output ("BOOK DEPTHS: bid=$($book.bid_depth) ask=$($book.ask_depth) (bucketed)")

    Write-Output "SMOKE TEST PASSED"
} catch {
    Write-Output ("HEALTH FAILED: " + $_.Exception.Message)
    Write-Output "SMOKE TEST FAILED"
    exit 1
} finally {
    Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    Write-Output "--- STDOUT ---"
    Get-Content "daemon_out.log" -ErrorAction SilentlyContinue
    Write-Output "--- STDERR ---"
    Get-Content "daemon_err.log" -ErrorAction SilentlyContinue
}