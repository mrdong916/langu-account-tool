$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
pnpm dev
exit $LASTEXITCODE
