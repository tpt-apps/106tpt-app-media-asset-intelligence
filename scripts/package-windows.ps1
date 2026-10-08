# Packs a Windows release artifact: the CLI binary + LICENSE + README into a
# versioned zip under target/release/dist. Run from the repo root.
#
#   pwsh scripts\package-windows.ps1
#
# Uses the release build only; leaves the dev profile untouched. Part of the
# §26 step 25 packaging path shared by the GitHub release workflow.

# NB: do NOT set $ErrorActionPreference='Stop' globally here — Windows
# PowerShell 5.1 turns cargo's stderr chatter (including a harmless
# "{{project-name}}" template warning) into a terminating error. We check
# $LASTEXITCODE explicitly instead.

$version = (Get-Content Cargo.toml | Select-String '^version = "([^"]+)"').Matches[0].Groups[1].Value
$name = "tpt-app-media-asset-intelligence-cli"
$bin = "tpt-media-asset-intel"
$release = Join-Path (Get-Location) "target\release"
$dist = Join-Path $release "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Write-Host "Building $name release (v$version) ..."
& cargo build --release -p $name 2>$null
if ($LASTEXITCODE -ne 0) { throw "release build failed with exit code $LASTEXITCODE" }

$staging = Join-Path $dist $bin
New-Item -ItemType Directory -Force -Path $staging | Out-Null
Copy-Item (Join-Path $release "$bin.exe") $staging -Force
if (-not $?) { throw "copying binary failed" }
Copy-Item LICENSE-MIT $staging -Force -ErrorAction SilentlyContinue
Copy-Item LICENSE-APACHE $staging -Force -ErrorAction SilentlyContinue
Copy-Item README.md $staging -Force -ErrorAction SilentlyContinue

$zip = Join-Path $dist ("{0}-v{1}-windows-x86_64.zip" -f $bin, $version)
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $staging "*") -DestinationPath $zip
if (-not $?) { throw "compressing artifact failed" }
Remove-Item $staging -Recurse -Force

Write-Host "Packaged $zip"