# Builds the release binary and packages it into a per-user NSIS installer:
#   dist\Switche-Setup-<version>.exe
# Requires NSIS (makensis), either on PATH or in Program Files (x86)\NSIS.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')

if (-not $env:GPUI_FXC_PATH) {
    $fxc = "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.0.22000.0\x64\fxc.exe"
    if (Test-Path $fxc) { $env:GPUI_FXC_PATH = $fxc }
}

Write-Host 'Building release binary ...'
cargo build --release --manifest-path switche\Cargo.toml
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$cargoToml = Get-Content switche\Cargo.toml
$version = ([regex]::Match(($cargoToml -join "`n"), '(?m)^version\s*=\s*"([^"]+)"')).Groups[1].Value
if (-not $version) { throw 'could not read version from switche\Cargo.toml' }

$exe = Join-Path (Get-Location) 'switche\target\release\Switche.exe'
if (-not (Test-Path $exe)) { throw "release binary not found at $exe" }
$sizeKb = [int]((Get-Item $exe).Length / 1KB)

$outDir = Join-Path (Get-Location) 'dist'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$outFile = Join-Path $outDir "Switche-Setup-$version.exe"

$makensis = "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
if (-not (Test-Path $makensis)) {
    $cmd = Get-Command makensis -ErrorAction SilentlyContinue
    if (-not $cmd) { throw 'makensis not found .. install NSIS or add it to PATH' }
    $makensis = $cmd.Source
}

Write-Host "Packaging $version ..."
& $makensis /V2 `
    "/DAPP_VERSION=$version" `
    "/DAPP_SIZE_KB=$sizeKb" `
    "/DSRC_EXE=$exe" `
    "/DICON=$PSScriptRoot\..\switche\switche.ico" `
    "/DOUTFILE=$outFile" `
    "$PSScriptRoot\switche.nsi"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "`nBuilt: $outFile"
