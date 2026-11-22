[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Container })]
    [string]$BenchmarkDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-FieldValue {
    param(
        [object]$Fields,
        [string]$Name
    )

    if ($null -eq $Fields) {
        return $null
    }

    $property = $Fields.PSObject.Properties[$Name]
    if ($null -ne $property) {
        return $property.Value
    }

    return $null
}

function ConvertTo-NullableDouble {
    param([object]$Value)

    if ($null -eq $Value -or $Value -is [bool]) {
        return $null
    }

    $parsed = 0.0
    $style = [System.Globalization.NumberStyles]::Float
    $culture = [System.Globalization.CultureInfo]::InvariantCulture
    if ([double]::TryParse([string]$Value, $style, $culture, [ref]$parsed)) {
        if ([double]::IsNaN($parsed) -or [double]::IsInfinity($parsed)) {
            return $null
        }
        return $parsed
    }

    return $null
}

function Get-Distribution {
    param([double[]]$Values)

    if ($null -eq $Values -or $Values.Count -eq 0) {
        return $null
    }

    $sorted = @($Values | Sort-Object)
    $count = $sorted.Count
    $sum = 0.0
    foreach ($value in $sorted) {
        $sum += $value
    }
    $mean = $sum / $count

    $squaredError = 0.0
    foreach ($value in $sorted) {
        $difference = $value - $mean
        $squaredError += $difference * $difference
    }

    $p50Index = [Math]::Max(0, [int][Math]::Ceiling(0.50 * $count) - 1)
    $p95Index = [Math]::Max(0, [int][Math]::Ceiling(0.95 * $count) - 1)
    $p99Index = [Math]::Max(0, [int][Math]::Ceiling(0.99 * $count) - 1)

    return [ordered]@{
        samples = $count
        mean = $mean
        stddev = [Math]::Sqrt($squaredError / $count)
        min = [double]$sorted[0]
        p50 = [double]$sorted[$p50Index]
        p95 = [double]$sorted[$p95Index]
        p99 = [double]$sorted[$p99Index]
        max = [double]$sorted[$count - 1]
    }
}

function Get-RunSummary {
    param([System.IO.DirectoryInfo]$RunDirectory)

    $metadataPath = Join-Path $RunDirectory.FullName "run.json"
    $logDirectory = Join-Path $RunDirectory.FullName "logs"
    if (-not (Test-Path -LiteralPath $metadataPath -PathType Leaf)) {
        throw "Metadados ausentes: $metadataPath"
    }
    if (-not (Test-Path -LiteralPath $logDirectory -PathType Container)) {
        throw "Diretório de logs ausente: $logDirectory"
    }

    $metadata = Get-Content -LiteralPath $metadataPath -Raw | ConvertFrom-Json
    $captureStart = [DateTimeOffset]::Parse([string]$metadata.capture_start_utc)
    $captureEnd = [DateTimeOffset]::Parse([string]$metadata.capture_end_utc)

    $metricNames = @(
        "ym2413_advance_elapsed_ns",
        "step_frame_elapsed_ns",
        "frame_processing_elapsed_ns",
        "audio_prepare_elapsed_ns",
        "audio_submit_elapsed_ns",
        "video_publish_elapsed_ns",
        "frame_interval_ns",
        "pacing_budget_ns",
        "pacing_wait_budget_ns",
        "fm_source_samples",
        "fm_finite_samples",
        "fm_unknown_samples",
        "fm_non_finite_samples",
        "fm_nonzero_samples",
        "fm_mixed_samples",
        "fm_peak",
        "fm_rms"
    )

    $metricValues = @{}
    foreach ($metricName in $metricNames) {
        $metricValues[$metricName] = [System.Collections.Generic.List[double]]::new()
    }

    $frameEvents = 0
    $overBudgetFrames = 0
    $frameBudgetSamples = 0
    $fallbackFrames = 0
    $fmSourceSamplesTotal = 0.0
    $fmUnknownSamplesTotal = 0.0
    $fmNonFiniteSamplesTotal = 0.0
    $fmDiagnosticsAvailable = $false
    $audioMetrics = $null
    $hostMetrics = $null
    $audioHighWaterFrames = 0.0
    $logFiles = @(Get-ChildItem -LiteralPath $logDirectory -File |
        Where-Object { $_.Name -like "sega-emulator.log*" } |
        Sort-Object LastWriteTimeUtc)

    if ($logFiles.Count -eq 0) {
        throw "Nenhum sega-emulator.log foi encontrado em $logDirectory"
    }

    foreach ($logFile in $logFiles) {
        foreach ($line in Get-Content -LiteralPath $logFile.FullName) {
            if ([string]::IsNullOrWhiteSpace($line)) {
                continue
            }

            try {
                $record = ConvertFrom-Json -InputObject $line -ErrorAction Stop
            }
            catch {
                continue
            }

            $timestampText = Get-FieldValue -Fields $record -Name "timestamp"
            if ($null -ne $timestampText) {
                try {
                    $timestamp = [DateTimeOffset]::Parse([string]$timestampText)
                    if ($timestamp -lt $captureStart -or $timestamp -gt $captureEnd) {
                        continue
                    }
                }
                catch {
                    continue
                }
            }

            $target = [string](Get-FieldValue -Fields $record -Name "target")
            $fields = Get-FieldValue -Fields $record -Name "fields"
            $eventName = [string](Get-FieldValue -Fields $fields -Name "event")
            $message = [string](Get-FieldValue -Fields $fields -Name "message")

            if ($target -eq "sega::audio::pacing" -and $message -eq "SMS frame pacing diagnostic") {
                $frameEvents += 1

                foreach ($metricName in $metricNames) {
                    $value = ConvertTo-NullableDouble (Get-FieldValue -Fields $fields -Name $metricName)
                    if ($null -ne $value) {
                        $metricValues[$metricName].Add([double]$value)
                    }
                }

                $processingNs = ConvertTo-NullableDouble (Get-FieldValue -Fields $fields -Name "frame_processing_elapsed_ns")
                $budgetNs = ConvertTo-NullableDouble (Get-FieldValue -Fields $fields -Name "pacing_budget_ns")
                if ($null -ne $processingNs -and $null -ne $budgetNs -and $budgetNs -gt 0) {
                    $frameBudgetSamples += 1
                    if ($processingNs -gt $budgetNs) {
                        $overBudgetFrames += 1
                    }
                }

                $fallback = Get-FieldValue -Fields $fields -Name "fm_fallback"
                if ($fallback -eq $true -or [string]$fallback -eq "true") {
                    $fallbackFrames += 1
                }

                $sourceField = Get-FieldValue -Fields $fields -Name "fm_source_samples"
                if ($null -ne $sourceField) {
                    $fmDiagnosticsAvailable = $true
                }
                $sourceSamples = ConvertTo-NullableDouble $sourceField
                $unknownSamples = ConvertTo-NullableDouble (Get-FieldValue -Fields $fields -Name "fm_unknown_samples")
                $nonFiniteSamples = ConvertTo-NullableDouble (Get-FieldValue -Fields $fields -Name "fm_non_finite_samples")
                if ($null -ne $sourceSamples) { $fmSourceSamplesTotal += $sourceSamples }
                if ($null -ne $unknownSamples) { $fmUnknownSamplesTotal += $unknownSamples }
                if ($null -ne $nonFiniteSamples) { $fmNonFiniteSamplesTotal += $nonFiniteSamples }
            }
            elseif ($target -eq "sega::audio::health" -and $eventName -eq "output_metrics") {
                $highWater = ConvertTo-NullableDouble (Get-FieldValue -Fields $fields -Name "queue_high_water_frames")
                if ($null -ne $highWater) {
                    $audioHighWaterFrames = [Math]::Max($audioHighWaterFrames, $highWater)
                }

                $audioMetrics = [ordered]@{
                    sample_rate_hz = Get-FieldValue -Fields $fields -Name "sample_rate_hz"
                    queue_capacity_frames = Get-FieldValue -Fields $fields -Name "queue_capacity_frames"
                    queue_queued_frames = Get-FieldValue -Fields $fields -Name "queue_queued_frames"
                    queue_high_water_frames = Get-FieldValue -Fields $fields -Name "queue_high_water_frames"
                    accepted_frames_total = Get-FieldValue -Fields $fields -Name "accepted_frames_total"
                    consumed_frames_total = Get-FieldValue -Fields $fields -Name "consumed_frames_total"
                    dropped_frames_total = Get-FieldValue -Fields $fields -Name "dropped_frames_total"
                    dropped_blocks_total = Get-FieldValue -Fields $fields -Name "dropped_blocks_total"
                    underrun_frames_total = Get-FieldValue -Fields $fields -Name "underrun_frames_total"
                    underrun_events_total = Get-FieldValue -Fields $fields -Name "underrun_events_total"
                    zero_fill_percent_total = Get-FieldValue -Fields $fields -Name "zero_fill_percent_total"
                    zero_fill_percent_window = Get-FieldValue -Fields $fields -Name "zero_fill_percent_window"
                    output_error = Get-FieldValue -Fields $fields -Name "output_error"
                }
            }
            elseif ($target -eq "sega::video::frame_pacing" -and $eventName -eq "host_metrics") {
                $hostMetrics = [ordered]@{
                    window_ms = Get-FieldValue -Fields $fields -Name "window_ms"
                    consumed = Get-FieldValue -Fields $fields -Name "consumed"
                    uploaded = Get-FieldValue -Fields $fields -Name "uploaded"
                    sequence_gaps = Get-FieldValue -Fields $fields -Name "sequence_gaps"
                    consumed_interval_p50_us = Get-FieldValue -Fields $fields -Name "consumed_interval_p50_us"
                    consumed_interval_p95_us = Get-FieldValue -Fields $fields -Name "consumed_interval_p95_us"
                    consumed_interval_p99_us = Get-FieldValue -Fields $fields -Name "consumed_interval_p99_us"
                    wake_interval_p50_us = Get-FieldValue -Fields $fields -Name "wake_interval_p50_us"
                    wake_interval_p95_us = Get-FieldValue -Fields $fields -Name "wake_interval_p95_us"
                    wake_interval_p99_us = Get-FieldValue -Fields $fields -Name "wake_interval_p99_us"
                }
            }
        }
    }

    if ($frameEvents -eq 0) {
        throw "$($RunDirectory.Name): nenhum evento 'SMS frame pacing diagnostic' foi encontrado dentro da janela de captura. Verifique consentimento e filtro de logs."
    }

    if ($null -ne $audioMetrics) {
        $sampleRate = ConvertTo-NullableDouble $audioMetrics.sample_rate_hz
        $queuedFrames = ConvertTo-NullableDouble $audioMetrics.queue_queued_frames
        $audioMetrics["queue_high_water_frames_observed"] = $audioHighWaterFrames
        $audioMetrics["queue_queued_ms"] = $null
        $audioMetrics["queue_high_water_ms"] = $null
        if ($null -ne $sampleRate -and $sampleRate -gt 0) {
            if ($null -ne $queuedFrames) {
                $audioMetrics["queue_queued_ms"] = 1000.0 * $queuedFrames / $sampleRate
            }
            $audioMetrics["queue_high_water_ms"] = 1000.0 * $audioHighWaterFrames / $sampleRate
        }
    }

    $distributions = [ordered]@{}
    foreach ($metricName in $metricNames) {
        $distributions[$metricName] = Get-Distribution -Values $metricValues[$metricName].ToArray()
    }

    $unknownPercent = 0.0
    if ($fmSourceSamplesTotal -gt 0) {
        $unknownPercent = 100.0 * ($fmUnknownSamplesTotal + $fmNonFiniteSamplesTotal) / $fmSourceSamplesTotal
    }

    $overBudgetPercent = $null
    if ($frameBudgetSamples -gt 0) {
        $overBudgetPercent = 100.0 * $overBudgetFrames / $frameBudgetSamples
    }
    $frameIntervalMeanMs = $null
    $workerFrameRateEstimateHz = $null
    $frameIntervalDistribution = $distributions["frame_interval_ns"]
    if ($null -ne $frameIntervalDistribution -and $frameIntervalDistribution.mean -gt 0) {
        $frameIntervalMeanMs = $frameIntervalDistribution.mean / 1000000.0
        $workerFrameRateEstimateHz = 1000000000.0 / $frameIntervalDistribution.mean
    }

    $fallbackPercent = $null
    $invalidPercent = $null
    $reportedFallbackFrames = $null
    $reportedSourceSamples = $null
    $reportedUnknownSamples = $null
    $reportedNonFiniteSamples = $null
    if ($fmDiagnosticsAvailable) {
        $fallbackPercent = 100.0 * $fallbackFrames / $frameEvents
        $invalidPercent = $unknownPercent
        $reportedFallbackFrames = $fallbackFrames
        $reportedSourceSamples = $fmSourceSamplesTotal
        $reportedUnknownSamples = $fmUnknownSamplesTotal
        $reportedNonFiniteSamples = $fmNonFiniteSamplesTotal
    }

    return [ordered]@{
        run = $RunDirectory.Name
        build = $metadata.build
        commit = $metadata.commit
        rom_file = $metadata.rom_file
        rom_sha256 = $metadata.rom_sha256
        hardware_profile = $metadata.hardware_profile
        fm_mode = $metadata.fm_mode
        capture_start_utc = $metadata.capture_start_utc
        capture_end_utc = $metadata.capture_end_utc
        capture_seconds = $metadata.capture_seconds
        frame_events = $frameEvents
        frame_processing_over_budget_frames = $overBudgetFrames
        frame_processing_over_budget_percent = $overBudgetPercent
        worker_frame_interval_mean_ms = $frameIntervalMeanMs
        worker_frame_rate_estimate_hz = $workerFrameRateEstimateHz
        fm_diagnostics_available = $fmDiagnosticsAvailable
        fm_fallback_frames = $reportedFallbackFrames
        fm_fallback_percent = $fallbackPercent
        fm_source_samples_total = $reportedSourceSamples
        fm_unknown_samples_total = $reportedUnknownSamples
        fm_non_finite_samples_total = $reportedNonFiniteSamples
        fm_invalid_sample_percent = $invalidPercent
        audio_queue_high_water_frames_observed = $audioHighWaterFrames
        metrics = $distributions
        audio_output = $audioMetrics
        host = $hostMetrics
        note = "frame_interval/worker metrics are not physical monitor scanout FPS; FM peak/RMS are digital source values before final device conversion."
    }
}

$benchmarkRoot = (Resolve-Path -LiteralPath $BenchmarkDirectory).Path
$manifestPath = Join-Path $benchmarkRoot "manifest.json"
if (-not (Test-Path -LiteralPath $manifestPath -PathType Leaf)) {
    throw "Manifest A/B ausente: $manifestPath"
}
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$requiredRuns = @("baseline-run1", "candidate-run1", "candidate-run2", "baseline-run2")
$runSummaries = [System.Collections.Generic.List[object]]::new()
$rawValuesByRun = @{}

foreach ($runName in $requiredRuns) {
    $runDirectory = Get-Item -LiteralPath (Join-Path $benchmarkRoot $runName) -ErrorAction Stop
    $summary = Get-RunSummary -RunDirectory $runDirectory
    $runSummaries.Add($summary)
    $rawValuesByRun[$runName] = $summary
}

$comparison = [System.Collections.Generic.List[object]]::new()
$comparisonMetrics = @(
    "ym2413_advance_elapsed_ns",
    "step_frame_elapsed_ns",
    "frame_processing_elapsed_ns",
    "audio_prepare_elapsed_ns",
    "audio_submit_elapsed_ns",
    "frame_interval_ns",
    "fm_unknown_samples",
    "fm_non_finite_samples",
    "fm_peak",
    "fm_rms"
)

foreach ($metricName in $comparisonMetrics) {
    $baselineValues = [System.Collections.Generic.List[double]]::new()
    $candidateValues = [System.Collections.Generic.List[double]]::new()

    foreach ($runName in @("baseline-run1", "baseline-run2")) {
        $distribution = $rawValuesByRun[$runName].metrics[$metricName]
        if ($null -ne $distribution) {
            $baselineValues.Add([double]$distribution.mean)
        }
    }
    foreach ($runName in @("candidate-run1", "candidate-run2")) {
        $distribution = $rawValuesByRun[$runName].metrics[$metricName]
        if ($null -ne $distribution) {
            $candidateValues.Add([double]$distribution.mean)
        }
    }

    if ($baselineValues.Count -eq 0 -or $candidateValues.Count -eq 0) {
        continue
    }

    $baselineMean = ($baselineValues | Measure-Object -Average).Average
    $candidateMean = ($candidateValues | Measure-Object -Average).Average
    $delta = $candidateMean - $baselineMean
    $deltaPercent = $null
    if ([Math]::Abs($baselineMean) -gt 1.0e-12) {
        $deltaPercent = 100.0 * $delta / [Math]::Abs($baselineMean)
    }

    $comparison.Add([ordered]@{
        metric = $metricName
        baseline_mean_of_run_means = $baselineMean
        candidate_mean_of_run_means = $candidateMean
        delta = $delta
        delta_percent = $deltaPercent
        interpretation = "Descriptive comparison only; inspect per-run distributions and audio counters before claiming a performance change."
    })
}

$summaryDocument = [ordered]@{
    schema = "sun-ym2413-ab-summary-v1"
    benchmark_directory = $benchmarkRoot
    generated_at_utc = [DateTimeOffset]::UtcNow.ToString("o")
    comparison_mode = $manifest.comparison_mode
    baseline_commit = $manifest.baseline_commit
    candidate_commit = $manifest.candidate_commit
    run_order = $requiredRuns
    runs = @($runSummaries)
    comparison = @($comparison)
    limitations = @(
        "The four runs are an ABBA comparison; compare both repetitions and dispersion, not only pooled means.",
        "Worker/frame intervals and host upload metrics do not prove physical display scanout at 60 Hz.",
        "FIFO capacity/high-water are not end-to-end audio latency measurements.",
        "FM peak/RMS describe the digital source before the physical output chain, not acoustic loudness or MO/RO voltage."
    )
}

$jsonPath = Join-Path $benchmarkRoot "summary.json"
$csvPath = Join-Path $benchmarkRoot "summary.csv"
$comparisonCsvPath = Join-Path $benchmarkRoot "comparison.csv"

$summaryDocument | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $jsonPath -Encoding UTF8

$csvRows = [System.Collections.Generic.List[object]]::new()
foreach ($run in $runSummaries) {
    foreach ($metricName in $run.metrics.Keys) {
        $distribution = $run.metrics[$metricName]
        if ($null -eq $distribution) {
            continue
        }
        $csvRows.Add([pscustomobject]@{
            run = $run.run
            build = $run.build
            commit = $run.commit
            rom_sha256 = $run.rom_sha256
            metric = $metricName
            samples = $distribution.samples
            mean = $distribution.mean
            stddev = $distribution.stddev
            min = $distribution.min
            p50 = $distribution.p50
            p95 = $distribution.p95
            p99 = $distribution.p99
            max = $distribution.max
        })
    }
}
$csvRows | Export-Csv -LiteralPath $csvPath -NoTypeInformation -Encoding UTF8
$comparison | ForEach-Object { [pscustomobject]$_ } |
    Export-Csv -LiteralPath $comparisonCsvPath -NoTypeInformation -Encoding UTF8

Write-Host "Resumo A/B gerado:" -ForegroundColor Green
Write-Host "  JSON: $jsonPath"
Write-Host "  CSV:  $csvPath"
Write-Host "  Comparação: $comparisonCsvPath"
Write-Host "Os campos de frame/worker não representam FPS de scanout do monitor."
