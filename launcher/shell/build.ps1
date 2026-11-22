#Requires -Version 5.1
param([switch]$Release)

$ErrorActionPreference = 'Stop'
$shellDir = $PSScriptRoot
$launcher = Split-Path -Parent $shellDir
$pack = Join-Path (Split-Path -Parent $launcher) 'pack'
$dist = Join-Path $launcher 'dist'
$frontend = Join-Path $launcher 'frontend'

New-Item -ItemType Directory -Force (Join-Path $dist 'shared\styles'), (Join-Path $dist 'shared\assets\fonts') | Out-Null
Copy-Item (Join-Path $pack 'common.css'), (Join-Path $pack 'dashboard.css'), (Join-Path $pack 'schemes.json') (Join-Path $dist 'shared')
Copy-Item (Join-Path $pack 'styles\fonts.css'), (Join-Path $pack 'styles\dashboard-*.css') (Join-Path $dist 'shared\styles')
Copy-Item (Join-Path $pack 'assets\fonts\*.woff2') (Join-Path $dist 'shared\assets\fonts')

npm --prefix $frontend run build
if ($LASTEXITCODE -ne 0) { throw 'Building Stellaria Shell frontend failed.' }

$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path $cargo)) { $cargo = 'cargo' }
$manifest = Join-Path $launcher 'Cargo.toml'
[string[]]$buildMode = @(); if ($Release) { $buildMode = @('--release') }

& $cargo build --offline --manifest-path $manifest -p stellaria-shell @buildMode
if ($LASTEXITCODE -ne 0) { throw 'Building stellaria-shell failed.' }
