# O instalador do porte Windows: o router.exe de release como sidecar, o front
# e o app em release, e o instalador NSIS — conferido no fim.
# Uso: .\scripts\build.ps1   (de qualquer pasta; imprime onde o instalador ficou)
$ErrorActionPreference = "Stop"

# O cargo pode não estar no PATH, e o `tauri build` também o chama: o do rustup
# entra no PATH só durante o build.
$pathBefore = $env:PATH
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
}

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $started = Get-Date
    $conf = Get-Content app\src-tauri\tauri.conf.json -Raw | ConvertFrom-Json

    Write-Host "== router.exe (release) ==" -ForegroundColor Cyan
    cargo build --release -p router-cli
    if ($LASTEXITCODE -ne 0) { throw "o router.exe não compilou" }

    # O `externalBin` pede o nome com o alvo; o instalador o põe ao lado do
    # app como router.exe, que é onde o app o procura.
    $triple = ((rustc -vV) | Select-String '^host: (.+)$').Matches[0].Groups[1].Value
    $binaries = "app\src-tauri\binaries"
    New-Item -ItemType Directory -Force $binaries | Out-Null
    Copy-Item target\release\router.exe "$binaries\router-$triple.exe" -Force

    Push-Location app
    try {
        if (-not (Test-Path node_modules)) {
            npm ci --no-fund --no-audit
            if ($LASTEXITCODE -ne 0) { throw "npm ci falhou" }
        }
        # O sidecar vem de um config à parte: no tauri.conf.json ele quebraria
        # todo `cargo build` do app sem o arquivo (ver o agent.md do src-tauri).
        # O `tauri build` roda o `npm run build` (o front) antes do Rust. Ele
        # apaga e recria target\release\nsis: um processo com o diretório atual
        # lá dentro (um terminal, um shell de ferramenta) o faz falhar com
        # "arquivo em uso" (os error 32).
        Write-Host "== front + app (release) + instalador ==" -ForegroundColor Cyan
        npx tauri build --config src-tauri/tauri.installer.conf.json
        if ($LASTEXITCODE -ne 0) { throw "o tauri build falhou" }
    }
    finally {
        Pop-Location
    }

    # A conferência: o instalador é DESTE build, e o script que o NSIS compilou
    # leva o router.exe ao lado do exe do app, e os ganchos. (O `installer.nsi`
    # é do bundler do Tauri 2.11; se ele mudar de lugar, isto falha alto.)
    Write-Host "== conferência ==" -ForegroundColor Cyan
    $setup = "target\release\bundle\nsis\$($conf.productName)_$($conf.version)_x64-setup.exe"
    if (-not (Test-Path $setup) -or (Get-Item $setup).LastWriteTime -lt $started) {
        throw "o instalador deste build não apareceu em $setup"
    }
    $nsi = Get-Content target\release\nsis\x64\installer.nsi -Raw
    if ($nsi -notmatch [regex]::Escape('"/oname=router.exe"')) {
        throw "o instalador não leva o router.exe (sidecar fora do build?)"
    }
    if ($nsi -notmatch [regex]::Escape("!define MAINBINARYNAME `"$($conf.mainBinaryName)`"")) {
        throw "o exe do app no instalador não se chama $($conf.mainBinaryName).exe"
    }
    # Os ganchos que tiram do caminho um router.exe em uso (sessão aberta).
    if ($nsi -notmatch '(?m)^!include ".*\\installer-hooks\.nsh"') {
        throw "o instalador não leva os ganchos do installer-hooks.nsh"
    }

    $mb = { param($path) "{0:N1} MB" -f ((Get-Item $path).Length / 1MB) }
    Write-Host ("instalador:  {0}  ({1})" -f (Resolve-Path $setup), (& $mb $setup))
    Write-Host ("  app:       {0}.exe  ({1})" -f $conf.mainBinaryName, (& $mb "target\release\$($conf.mainBinaryName).exe"))
    Write-Host ("  router:    router.exe  ({0})" -f (& $mb target\release\router.exe))
    Write-Host "`npronto." -ForegroundColor Green
}
finally {
    Pop-Location
    $env:PATH = $pathBefore
}
