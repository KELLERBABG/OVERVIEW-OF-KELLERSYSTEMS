$policy = Get-Content "test-data/collision-policy.json" -Raw
$event = Get-Content "test-data/frontal-collision.json" -Raw

$body = @{
    event_json = $event
    policy_json = $policy
} | ConvertTo-Json -Depth 10

Write-Output "=== Creating Claim ==="
$claim = Invoke-RestMethod -Uri "http://localhost:8080/api/claims" -Method Post -Body $body -ContentType "application/json"
Write-Output "Created: $($claim.id)"
Write-Output "Policy: $($claim.policy_name)"
Write-Output "Status: $($claim.status)"
$id = $claim.id

Write-Output "`n=== Listing Claims ==="
$list = Invoke-RestMethod -Uri "http://localhost:8080/api/claims"
Write-Output "Claims: $($list.claims.Count)"

Write-Output "`n=== Verifying Claim ==="
$v = Invoke-RestMethod -Uri "http://localhost:8080/api/claims/$id/verify" -Method Post
Write-Output "Verdict: $($v.verdict)"

Write-Output "`n=== Classifying Claim ==="
$c = Invoke-RestMethod -Uri "http://localhost:8080/api/claims/$id/classify" -Method Post
Write-Output "Class: $($c.classification)"
Write-Output "Confidence: $($c.confidence)"

Write-Output "`n=== Taint Analysis ==="
$t = Invoke-RestMethod -Uri "http://localhost:8080/api/claims/$id/taint" -Method Post
Write-Output "Trust Score: $($t.trust_score)"
Write-Output "Taint Verdict: $($t.taint_verdict)"

Write-Output "`n=== Damage Report ==="
$r = Invoke-RestMethod -Uri "http://localhost:8080/api/claims/$id/report" -Method Post
Write-Output "Report ID: $($r.report_id)"
Write-Output "Classification: $($r.classification.label)"
Write-Output "Confidence: $($r.classification.confidence)"
Write-Output "Readings: $($r.readings.total)"

Write-Output "`n=== Dashboard URL ==="
Write-Output "Open http://localhost:8080/ in your browser"
