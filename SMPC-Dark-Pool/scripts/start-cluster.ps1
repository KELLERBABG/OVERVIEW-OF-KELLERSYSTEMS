#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Starts the SMPC Dark Pool cluster with Docker Compose.
.DESCRIPTION
    Builds and starts all 5 MPC nodes plus a Ganache Ethereum testnet
    using docker compose.
.EXAMPLE
    .\scripts\start-cluster.ps1
#>

$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $ProjectRoot

Write-Host "Starting SMPC Dark Pool cluster..." -ForegroundColor Cyan
Write-Host "Nodes: node0, node1, node2, node3, node4" -ForegroundColor Cyan
Write-Host "Ethereum testnet: ganache (chain ID 1337)" -ForegroundColor Cyan
Write-Host ""

docker compose up --build
