[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$RomPath,

    [string]$OutputDirectory = "",

    [ValidateRange(10, 600)]
    [int]$WarmupSeconds = 30,

    [ValidateRange(60, 1800)]
    [int]$CaptureSeconds = 180
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "Este roteiro exige Windows x64."
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git não foi encontrado no PATH."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo não foi encontrado no PATH."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$rom = Get-Item -LiteralPath (Resolve-Path -LiteralPath $RomPath).Path
$romHash = (Get-FileHash -LiteralPath $rom.FullName -Algorithm SHA256).Hash.ToLowerInvariant()

$baselineSha = "c8a38e9c4aee33de999547c328be579b5e3476e8"
$candidateSha = "8b4ea7fbb3ceb7e293aafae9cfef87d3ccde687b"
$logFilter = "info,sega::audio::pacing=trace,sega::video::frame_pacing=trace,sega::audio::health=trace"

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $documents = [Environment]::GetFolderPath("MyDocuments")
    $OutputDirectory = Join-Path $documents "SUN-Benchmarks\ym2413-real-game-ab"
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outputRoot = Join-Path $OutputDirectory $stamp
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) "SUN-YM2413-AB-$stamp"
$baselineWorktree = Join-Path $tempRoot "baseline"
$candidateWorktree = Join-Path $tempRoot "candidate"

New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

function Invoke-GitChecked {
    param([string[]]$GitArgs)
    & git -C $repoRoot @GitArgs
    if ($LASTEXITCODE -ne 0) {
        throw "Falha no comando git: git $($GitArgs -join ' ')"
    }
}

function Build-Desktop {
    param(
        [string]$Label,
        [string]$Worktree,
        [string]$Commit
    )

    $buildRoot = Join-Path $outputRoot "$Label-build"
    $targetRoot = Join-Path $buildRoot "target"
    $buildLog = Join-Path $buildRoot "build.log"
    New-Item -ItemType Directory -Force -Path $buildRoot | Out-Null

    Write-Host ""
    Write-Host "=== Build $Label ($Commit) ===" -ForegroundColor Cyan
    $previousTarget = $env:CARGO_TARGET_DIR
    try {
        $env:CARGO_TARGET_DIR = $targetRoot
        Push-Location $Worktree
        try {
            & cargo build --release --locked --manifest-path native/sega_desktop/Cargo.toml 2>&1 |
                Tee-Object -FilePath $buildLog |
                Out-Host
            if ($LASTEXITCODE -ne 0) {
                throw "Build Release falhou para $Label. Consulte $buildLog"
            }
        }
        finally {
            Pop-Location
        }
    }
    finally {
        if ($null -eq $previousTarget) {
            Remove-Item Env:CARGO_TARGET_DIR -ErrorAction SilentlyContinue
        }
        else {
            $env:CARGO_TARGET_DIR = $previousTarget
        }
    }

    $exe = Join-Path $targetRoot "release\sega-desktop.exe"
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
        throw "Executável não encontrado após o build: $exe"
    }
    return $exe
}

function Invoke-MeasurementRun {
    param(
        [string]$Label,
        [int]$Index,
        [string]$Worktree,
        [string]$Executable,
        [string]$Commit
    )

    $runName = "{0}-run{1}" -f $Label, $Index
    $runRoot = Join-Path $outputRoot $runName
    $logRoot = Join-Path $runRoot "logs"
    New-Item -ItemType Directory -Force -Path $logRoot | Out-Null

    Write-Host ""
    Write-Host "=== $runName ===" -ForegroundColor Cyan
    Write-Host "ROM: $($rom.Name)"
    Write-Host "SHA-256: $romHash"
    Write-Host "Build: $Commit"
    Write-Host "Configure/confirm: SMS1, região Japão, placa Japanese SMS / YM2413 integrado, FM On."
    Write-Host "Mantenha volume, DSP, resolução, escala, VSync/VRR e cena do jogo iguais em todas as execuções."
    Write-Host "No menu DEBUG, autorize logs e métricas. NÃO ative 'Modo Debugger' nem trace de barramento."
    Write-Host "O aplicativo abrirá com a mesma ROM e região Japão."

    $previousLogDir = $env:SEGA_LOG_DIR
    $previousFilter = $env:SEGA_LOG_FILTER
    try {
        $env:SEGA_LOG_DIR = $logRoot
        $env:SEGA_LOG_FILTER = $logFilter
        $arguments = '"' + $rom.FullName + '" japan'
        $process = Start-Process -FilePath $Executable -ArgumentList $arguments -WorkingDirectory $Worktree -PassThru
    }
    finally {
        if ($null -eq $previousLogDir) {
            Remove-Item Env:SEGA_LOG_DIR -ErrorAction SilentlyContinue
        }
        else {
            $env:SEGA_LOG_DIR = $previousLogDir
        }
        if ($null -eq $previousFilter) {
            Remove-Item Env:SEGA_LOG_FILTER -ErrorAction SilentlyContinue
        }
        else {
            $env:SEGA_LOG_FILTER = $previousFilter
        }
    }

    [void](Read-Host "Com o jogo na cena escolhida e a coleta diagnóstica autorizada, pressione ENTER para iniciar o aquecimento")
    $process.Refresh()
    if ($process.HasExited) {
        throw "${runName}: o emulador encerrou antes da captura."
    }

    Start-Sleep -Seconds $WarmupSeconds
    $captureStart = [DateTimeOffset]::UtcNow
    Write-Host "CAPTURA INICIADA: $($captureStart.ToString('o'))" -ForegroundColor Green
    Start-Sleep -Seconds $CaptureSeconds
    $captureEnd = [DateTimeOffset]::UtcNow
    Write-Host "CAPTURA ENCERRADA: $($captureEnd.ToString('o'))" -ForegroundColor Green

    $process.Refresh()
    if ($process.HasExited) {
        throw "${runName}: o emulador encerrou durante a captura."
    }
    [void](Read-Host "Feche o emulador normalmente para descarregar o arquivo de log; pressione ENTER depois de fechá-lo")
    $process.WaitForExit()

    $logFiles = @(Get-ChildItem -LiteralPath $logRoot -File)
    if ($logFiles.Count -eq 0) {
        throw "${runName}: nenhum log foi gerado. Confirme a autorização diagnóstica."
    }

    $archive = Join-Path $runRoot "logs.zip"
    Compress-Archive -Path (Join-Path $logRoot "*") -DestinationPath $archive -Force

    $metadata = [ordered]@{
        run = $runName
        build = $Label
        commit = $Commit
        rom_file = $rom.Name
        rom_sha256 = $romHash
        region = "japan"
        hardware_profile = "SMS1 / Japanese integrated YM2413"
        fm_mode = "on"
        warmup_seconds = $WarmupSeconds
        capture_seconds = $CaptureSeconds
        capture_start_utc = $captureStart.ToString("o")
        capture_end_utc = $captureEnd.ToString("o")
        log_filter = $logFilter
        log_files = @($logFiles | ForEach-Object { $_.Name })
    }
    $metadata | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $runRoot "run.json") -Encoding UTF8

    Write-Host "Evidências salvas em: $runRoot" -ForegroundColor Green
}

$worktreesAdded = [System.Collections.Generic.List[string]]::new()
try {
    Write-Host "Atualizando refs e verificando os commits A/B..."
    Invoke-GitChecked -GitArgs @("fetch", "origin", "main")

    $dirty = & git -C $repoRoot status --porcelain
    if ($LASTEXITCODE -ne 0) {
        throw "Não foi possível verificar o estado do repositório."
    }
    if (-not [string]::IsNullOrWhiteSpace(($dirty -join ""))) {
        throw "O worktree principal possui alterações locais. Faça commit/stash antes do A/B."
    }

    New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

    Invoke-GitChecked -GitArgs @("cat-file", "-e", "$baselineSha^{commit}")
    Invoke-GitChecked -GitArgs @("cat-file", "-e", "$candidateSha^{commit}")

    $changedPaths = @(& git -C $repoRoot diff --name-only $baselineSha $candidateSha)
    if ($LASTEXITCODE -ne 0) {
        throw "Não foi possível comparar os commits A/B."
    }
    if ($changedPaths.Count -ne 1 -or $changedPaths[0] -ne "native/ym2413/src/pg_serial.rs") {
        throw "Os commits não são um A/B isolado de pg_serial.rs: $($changedPaths -join ', ')"
    }

    Invoke-GitChecked -GitArgs @("worktree", "add", "--detach", $baselineWorktree, $baselineSha)
    $worktreesAdded.Add($baselineWorktree)
    Invoke-GitChecked -GitArgs @("worktree", "add", "--detach", $candidateWorktree, $candidateSha)
    $worktreesAdded.Add($candidateWorktree)

    $baselineExe = Build-Desktop -Label "baseline" -Worktree $baselineWorktree -Commit $baselineSha
    $candidateExe = Build-Desktop -Label "candidate" -Worktree $candidateWorktree -Commit $candidateSha

    $manifest = [ordered]@{
        baseline_commit = $baselineSha
        candidate_commit = $candidateSha
        changed_files = $changedPaths
        rom_file = $rom.Name
        rom_sha256 = $romHash
        warmup_seconds = $WarmupSeconds
        capture_seconds = $CaptureSeconds
        run_order = @("baseline-run1", "candidate-run1", "candidate-run2", "baseline-run2")
        log_filter = $logFilter
        note = "A/B isolates the single pg_serial.rs optimization from PR #1199; both builds include the in-place runtime path from PR #1194."
    }
    $manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputRoot "manifest.json") -Encoding UTF8

    Invoke-MeasurementRun -Label "baseline" -Index 1 -Worktree $baselineWorktree -Executable $baselineExe -Commit $baselineSha
    Invoke-MeasurementRun -Label "candidate" -Index 1 -Worktree $candidateWorktree -Executable $candidateExe -Commit $candidateSha
    Invoke-MeasurementRun -Label "candidate" -Index 2 -Worktree $candidateWorktree -Executable $candidateExe -Commit $candidateSha
    Invoke-MeasurementRun -Label "baseline" -Index 2 -Worktree $baselineWorktree -Executable $baselineExe -Commit $baselineSha

    $bundle = Join-Path $OutputDirectory "ym2413-real-game-ab-$stamp.zip"
    Compress-Archive -Path (Join-Path $outputRoot "*") -DestinationPath $bundle -Force

    Write-Host ""
    Write-Host "A/B concluído. Envie o pacote abaixo para análise:" -ForegroundColor Green
    Write-Host $bundle
    Write-Host "Pasta de evidências: $outputRoot"
}
finally {
    foreach ($worktree in $worktreesAdded) {
        & git -C $repoRoot worktree remove --force $worktree 2>$null
    }
    if (Test-Path -LiteralPath $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}
