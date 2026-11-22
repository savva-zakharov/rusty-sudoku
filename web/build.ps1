# Build the web version into dist/ (PowerShell version of web/build.sh).
# In PowerShell, `bash` is usually WSL's bash, whose Rust may lack the wasm
# target; this script uses the Windows toolchain instead.
#   powershell -ExecutionPolicy Bypass -File web/build.ps1
$ErrorActionPreference = "Stop"
Set-Location (Split-Path -Parent $PSScriptRoot)

cargo build --release --target wasm32-unknown-unknown
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

if (Test-Path dist) { Remove-Item -Recurse -Force dist }
New-Item -ItemType Directory -Force dist/assets/palettes, dist/assets/sounds | Out-Null
Copy-Item web/index.html, web/gl.js, web/audio.js, web/net.js dist/
Copy-Item target/wasm32-unknown-unknown/release/rusty-sudoku.wasm dist/
Copy-Item assets/palettes.txt dist/assets/
foreach ($line in Get-Content assets/palettes.txt) {
    $name = ($line -replace '#.*', '').Trim()
    if ($name -match '^[A-Za-z0-9_-]+$' -and (Test-Path "assets/palettes/$name.png")) {
        Copy-Item "assets/palettes/$name.png" dist/assets/palettes/
    }
}
Copy-Item assets/sounds/*.wav dist/assets/sounds/

$size = (Get-ChildItem dist -Recurse -File | Measure-Object Length -Sum).Sum
"built dist/ ({0:N1} MB)" -f ($size / 1MB)
