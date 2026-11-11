<#
.SYNOPSIS
    Synchronizes all project repositories from SYSTEMS & CREATIONS into
    OVERVIEW-OF-KELLERSYSTEMS, recalculates language metrics, and pushes to GitHub.
#>
[CmdletBinding()]
param(
    [string]$SourceDir = "C:\Users\lukas\Downloads\SYSTEMS & CREATIONS",
    [string]$DestDir = "C:\Users\lukas\Downloads\OVERVIEW-OF-KELLERSYSTEMS",
    [string]$CommitMessage = "sync: update KellerSystems ecosystem & language stats"
)

$ErrorActionPreference = "Stop"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host "   KELLERSYSTEMS OVERVIEW SYNCHRONIZER & METRICS ENGINE   " -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

if (-not (Test-Path $SourceDir)) {
    Write-Error "Source directory not found: $SourceDir"
}

if (-not (Test-Path $DestDir)) {
    New-Item -ItemType Directory -Path $DestDir -Force | Out-Null
}

$excludeDirs = @('.git', 'target', 'node_modules', '.venv', 'venv', 'env', '.keller', 'dist', 'build', 'obj', 'bin', '_git-sync', 'design-atlas-plugin', 'roblox-game', '.idea', '.vscode')
$excludeFiles = @('*.pdb', '*.iso', '*.exe', '*.dll', '*.so', '*.dylib', '*.wasm', '*.bin', '*.db', '*.db-wal', '*.db-shm', '*.pyc', '*.coverage')

# 1. Discover all projects
$projects = Get-ChildItem -Path $SourceDir -Directory | Where-Object { 
    $_.Name -notin @('_git-sync', 'design-atlas-plugin', 'roblox-game') -and (Test-Path (Join-Path $_.FullName ".git"))
}

Write-Host "[1/4] Mirroring $($projects.Count) project directories..." -ForegroundColor Yellow

foreach ($proj in $projects) {
    $pSource = $proj.FullName
    $pDest = Join-Path $DestDir $proj.Name
    robocopy $pSource $pDest /MIR /XD $excludeDirs /XF $excludeFiles /R:1 /W:1 /NFL /NDL /NJH /NJS /NC /NS /NP | Out-Null
}

Write-Host "[2/4] Recalculating Language Breakdown..." -ForegroundColor Yellow

$langMap = @{
    '.rs' = 'Rust'
    '.py' = 'Python'
    '.c' = 'C/C++'
    '.h' = 'C/C++'
    '.cpp' = 'C/C++'
    '.hpp' = 'C/C++'
    '.ino' = 'C/C++ (Arduino)'
    '.sol' = 'Solidity'
    '.circom' = 'Circom'
    '.ts' = 'TypeScript'
    '.tsx' = 'TypeScript'
    '.js' = 'JavaScript'
    '.mjs' = 'JavaScript'
    '.sh' = 'Shell'
    '.ps1' = 'PowerShell'
}

$files = Get-ChildItem -Path $DestDir -Recurse -File | Where-Object {
    $langMap.ContainsKey($_.Extension.ToLower())
}

$rows = @()
$grouped = $files | Group-Object { $langMap[$_.Extension.ToLower()] }
$totalBytes = ($files | Measure-Object -Property Length -Sum).Sum

foreach ($g in $grouped) {
    $bytes = ($g.Group | Measure-Object -Property Length -Sum).Sum
    $lines = 0
    foreach ($f in $g.Group) {
        $lines += [System.IO.File]::ReadAllLines($f.FullName).Count
    }
    $rows += [PSCustomObject]@{
        Language = $g.Name
        Files = $g.Count
        Lines = $lines
        SizeMB = [Math]::Round($bytes / 1MB, 2)
        PctBytes = [Math]::Round(($bytes / $totalBytes) * 100, 2)
    }
}

$totalLines = ($rows | Measure-Object -Property Lines -Sum).Sum
$sortedRows = $rows | Select-Object Language, Files, Lines, @{Name="PctLines"; Expression={[Math]::Round(($_.Lines / $totalLines) * 100, 2)}}, SizeMB, PctBytes | Sort-Object PctBytes -Descending

Write-Host "`nLanguage Breakdown Results:" -ForegroundColor Green
$sortedRows | Format-Table -AutoSize | Out-String | Write-Host

Write-Host "[3/4] Staging Git Changes..." -ForegroundColor Yellow
git -C $DestDir add -A

$status = git -C $DestDir status --porcelain
if ($status) {
    Write-Host "[4/4] Committing & Pushing to GitHub..." -ForegroundColor Yellow
    git -C $DestDir commit -m "$CommitMessage [$(Get-Date -Format 'yyyy-MM-dd HH:mm')]"
    git -C $DestDir push -u origin main
    Write-Host "`nSuccessfully pushed to GitHub: https://github.com/KELLERBABG/OVERVIEW-OF-KELLERSYSTEMS" -ForegroundColor Green
} else {
    Write-Host "No changes detected. Working tree clean." -ForegroundColor Cyan
}
