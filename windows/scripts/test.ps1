# A verificação do porte Windows — a mesma que a CI roda.
# Uso: .\scripts\test.ps1   (a partir de windows\)
$ErrorActionPreference = "Stop"

# O cargo pode não estar no PATH; prefira o do rustup se existir.
$cargo = if (Get-Command cargo -ErrorAction SilentlyContinue) {
    "cargo"
} else {
    "$env:USERPROFILE\.cargo\bin\cargo.exe"
}

Push-Location (Join-Path $PSScriptRoot "..")
try {
    Write-Host "== fmt --check ==" -ForegroundColor Cyan
    & $cargo fmt --check
    if ($LASTEXITCODE -ne 0) { throw "cargo fmt --check falhou" }

    Write-Host "== clippy -D warnings ==" -ForegroundColor Cyan
    & $cargo clippy --workspace --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw "clippy falhou" }

    Write-Host "== test ==" -ForegroundColor Cyan
    & $cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "os testes falharam" }

    Write-Host "`ntudo verde." -ForegroundColor Green
}
finally {
    Pop-Location
}
