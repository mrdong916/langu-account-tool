$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
pnpm exec tauri icon src/assets/images/langu-logo.png --output work/company-icons
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Get-ChildItem -LiteralPath work/company-icons -File | Copy-Item -Destination src-tauri/icons -Force
