# Roda (ou compila) o jogo nativo no Windows a partir desta pasta do WSL.
# Uso (PowerShell do Windows):  .\tools\windows.ps1          -> cargo run --features dev
#                               .\tools\windows.ps1 build    -> só compila
# Os artefatos ficam no disco do Windows (bem mais rápido que dentro do WSL) e o
# paralelismo é baixo porque o WSL divide os ~8 GB de RAM com o build.
param([string]$cmd = "run")

$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
$env:CARGO_TARGET_DIR = "$env:USERPROFILE\cozy-target"
$env:CARGO_BUILD_JOBS = "2"
Set-Location (Split-Path $PSScriptRoot -Parent)
cargo $cmd --features dev
