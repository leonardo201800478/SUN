[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$parserPath = Join-Path $repositoryRoot "scripts\benchmark\summarize-ym2413-real-game-ab.ps1"
if (-not (Test-Path -LiteralPath $parserPath -PathType Leaf)) {
    throw "Parser não encontrado: $parserPath"
}

$benchmarkRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("SUN-YM2413-AB-Parser-" + [Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $benchmarkRoot -Force | Out-Null

function Write-JsonFile {
    param([string]$Path, [object]$Value)

    $Value | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Write-JsonLine {
    param(
        [string]$Path,
        [string]$Target,
        [object]$Fields,
        [string]$Timestamp
    )

    $record = [ordered]@{
        timestamp = $Timestamp
        level = "INFO"
        fields = $Fields
        target = $Target
    }
    $record | ConvertTo-Json -Depth 10 -Compress | Add-Content -LiteralPath $Path -Encoding UTF8
}

try {
    $captureStart = [DateTimeOffset]::UtcNow.AddMinutes(-1)
    $captureEnd = [DateTimeOffset]::UtcNow.AddMinutes(1)
    $timestamp = [DateTimeOffset]::UtcNow.ToString("o")
    $runOrder = @("baseline-run1", "candidate-run1", "candidate-run2", "baseline-run2")

    foreach ($runName in $runOrder) {
        $runDirectory = Join-Path $benchmarkRoot $runName
        $logDirectory = Join-Path $runDirectory "logs"
        New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null
        $logPath = Join-Path $logDirectory "sega-emulator.log"

        $isCandidate = $runName.StartsWith("candidate")
        if ($isCandidate) {
            $buildName = "candidate"
            $commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }
        else {
            $buildName = "baseline"
            $commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }

        $metadata = [ordered]@{
            build = $buildName
            commit = $commit
            rom_file = "Kenseiden (Japan).sms"
            rom_sha256 = "7a9c056edfb1b973482430a41074b0b0a076ee4591d5df8f19007e7058cadc93"
            hardware_profile = "SMS1"
            fm_mode = "on"
            capture_start_utc = $captureStart.ToString("o")
            capture_end_utc = $captureEnd.ToString("o")
            capture_seconds = 120
        }
        Write-JsonFile -Path (Join-Path $runDirectory "run.json") -Value $metadata

        foreach ($processingNs in @(17000000, 15000000)) {
            $frameFields = [ordered]@{
                message = "SMS frame pacing diagnostic"
                ym2413_advance_elapsed_ns = 120000
                step_frame_elapsed_ns = 16000000
                frame_processing_elapsed_ns = $processingNs
                audio_prepare_elapsed_ns = 250000
                audio_submit_elapsed_ns = 100000
                video_publish_elapsed_ns = 200000
                frame_interval_ns = 16666667
                pacing_budget_ns = 16666667
                pacing_wait_budget_ns = 0
            }

            if ($isCandidate) {
                $frameFields.fm_output_enabled = $true
                $frameFields.fm_source_samples = 100
                $frameFields.fm_finite_samples = 100
                $frameFields.fm_unknown_samples = 0
                $frameFields.fm_non_finite_samples = 0
                $frameFields.fm_nonzero_samples = 99
                $frameFields.fm_mixed_samples = 100
                $frameFields.fm_fallback = $false
                $frameFields.fm_peak = 0.5
                $frameFields.fm_rms = 0.2
            }

            Write-JsonLine -Path $logPath -Target "sega::audio::pacing" -Fields $frameFields -Timestamp $timestamp
        }

        $audioFields = [ordered]@{
            event = "output_metrics"
            sample_rate_hz = 48000
            queue_capacity_frames = 4096
            queue_queued_frames = 480
            queue_high_water_frames = 960
            accepted_frames_total = 1440000
            consumed_frames_total = 1440000
            dropped_frames_total = 0
            dropped_blocks_total = 0
            underrun_frames_total = 0
            underrun_events_total = 0
            zero_fill_percent_total = 0
            zero_fill_percent_window = 0
            output_error = $null
        }
        Write-JsonLine -Path $logPath -Target "sega::audio::health" -Fields $audioFields -Timestamp $timestamp

        $hostFields = [ordered]@{
            event = "host_metrics"
            window_ms = 1000
            consumed = 60
            uploaded = 60
            sequence_gaps = 0
            consumed_interval_p50_us = 16666
            consumed_interval_p95_us = 16670
            consumed_interval_p99_us = 16680
            wake_interval_p50_us = 16666
            wake_interval_p95_us = 16670
            wake_interval_p99_us = 16680
        }
        Write-JsonLine -Path $logPath -Target "sega::video::frame_pacing" -Fields $hostFields -Timestamp $timestamp
    }

    $manifest = [ordered]@{
        comparison_mode = "same-commit-repeatability"
        baseline_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        candidate_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        run_order = $runOrder
    }
    Write-JsonFile -Path (Join-Path $benchmarkRoot "manifest.json") -Value $manifest

    & $parserPath -BenchmarkDirectory $benchmarkRoot

    $summaryPath = Join-Path $benchmarkRoot "summary.json"
    $csvPath = Join-Path $benchmarkRoot "summary.csv"
    $comparisonPath = Join-Path $benchmarkRoot "comparison.csv"
    foreach ($outputPath in @($summaryPath, $csvPath, $comparisonPath)) {
        if (-not (Test-Path -LiteralPath $outputPath -PathType Leaf)) {
            throw "Saída esperada não foi criada: $outputPath"
        }
    }

    $summary = Get-Content -LiteralPath $summaryPath -Raw | ConvertFrom-Json
    if ($summary.runs.Count -ne 4) {
        throw "Esperadas quatro execuções no summary.json."
    }
    if ($summary.comparison_mode -ne "same-commit-repeatability") {
        throw "Modo de comparação não foi preservado no summary.json."
    }

    $baseline = $summary.runs | Where-Object { $_.run -eq "baseline-run1" }
    $candidate = $summary.runs | Where-Object { $_.run -eq "candidate-run1" }
    if ($baseline.fm_diagnostics_available -ne $false) {
        throw "Métricas FM ausentes no baseline devem ser marcadas como indisponíveis, não zero."
    }
    if ($candidate.fm_diagnostics_available -ne $true) {
        throw "Métricas FM do candidate não foram reconhecidas."
    }
    if ([Math]::Abs($candidate.frame_processing_over_budget_percent - 50.0) -gt 0.001) {
        throw "Percentual de frames acima do orçamento incorreto."
    }
    if ([Math]::Abs($candidate.worker_frame_rate_estimate_hz - 60.0) -gt 0.01) {
        throw "Estimativa de cadência do worker incorreta."
    }
    if ([Math]::Abs($candidate.audio_output.queue_queued_ms - 10.0) -gt 0.001) {
        throw "Conversão do backlog atual para milissegundos incorreta."
    }
    if ([Math]::Abs($candidate.audio_output.queue_high_water_ms - 20.0) -gt 0.001) {
        throw "Conversão do high-water para milissegundos incorreta."
    }

    Write-Host "YM2413 A/B statistics parser self-test passed." -ForegroundColor Green
}
finally {
    if (Test-Path -LiteralPath $benchmarkRoot) {
        Remove-Item -LiteralPath $benchmarkRoot -Recurse -Force
    }
}
