[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$RomPath,

    [string]$OutputDirectory = "",

    [ValidateRange(1, 600)]
    [int]$WarmupSeconds = 5,

    [ValidateRange(1, 1800)]
    [int]$CaptureSeconds = 30,

    [ValidateRange(1, 300)]
    [int]$DiagnosticSeconds = 5,

    [switch]$CaptureCpuProfile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "A campanha PERF-01/S1 ABBA exige Windows."
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git não foi encontrado no PATH."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo não foi encontrado no PATH."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$harness = Join-Path $PSScriptRoot "perf-baseline-s0.ps1"
if (-not (Test-Path -LiteralPath $harness -PathType Leaf)) {
    throw "Harness S0 não encontrado: $harness"
}
$rom = Get-Item -LiteralPath (Resolve-Path -LiteralPath $RomPath).Path
if ($rom.Name -notmatch "(?i)outrun") {
    throw "OutRun é o benchmark principal da PERF-01. ROM informada: $($rom.Name)"
}

$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Não foi possível identificar o commit."
}
$branch = (& git -C $repoRoot branch --show-current).Trim()
if (-not [string]::IsNullOrWhiteSpace((& git -C $repoRoot status --porcelain))) {
    throw "O working tree precisa estar limpo antes da campanha ABBA."
}
$romHash = (Get-FileHash -LiteralPath $rom.FullName -Algorithm SHA256).Hash.ToLowerInvariant()

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $documents = [Environment]::GetFolderPath("MyDocuments")
    $OutputDirectory = Join-Path $documents "SUN-Benchmarks\perf-01-abba"
}
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$runRoot = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) $stamp
New-Item -ItemType Directory -Path $runRoot -Force | Out-Null

# A-B-B-A order reduces monotonic thermal/load bias. Each profile reuses its own
# Cargo target directory, so only the first occurrence of each profile compiles.
$sequence = @(
    [ordered]@{ index = 1; label = "A1-release"; profile = "release" },
    [ordered]@{ index = 2; label = "B1-thinlto"; profile = "release-llvm" },
    [ordered]@{ index = 3; label = "B2-thinlto"; profile = "release-llvm" },
    [ordered]@{ index = 4; label = "A2-release"; profile = "release" }
)

$targetRoot = Join-Path $runRoot "cargo-target"
$releaseTarget = Join-Path $targetRoot "release-control"
$thinLtoTarget = Join-Path $targetRoot "release-llvm"
New-Item -ItemType Directory -Path $releaseTarget, $thinLtoTarget -Force | Out-Null

$allRows = [System.Collections.Generic.List[object]]::new()
$runRecords = [System.Collections.Generic.List[object]]::new()
$campaignLog = Join-Path $runRoot "abba.log"
$failure = $null

foreach ($step in $sequence) {
    $stepOutput = Join-Path $runRoot ("run-{0:D2}-{1}" -f $step.index, $step.label)
    New-Item -ItemType Directory -Path $stepOutput -Force | Out-Null
    $stepLog = Join-Path $stepOutput "runner.log"
    $targetDirectory = if ($step.profile -eq "release") { $releaseTarget } else { $thinLtoTarget }

    Write-Host ""
    Write-Host ("[{0}/4] {1} — profile {2}" -f $step.index, $step.label, $step.profile) -ForegroundColor Cyan

    $harnessArgs = @{
        RomPath = $rom.FullName
        OutputDirectory = $stepOutput
        WarmupSeconds = $WarmupSeconds
        CaptureSeconds = $CaptureSeconds
        DiagnosticSeconds = $DiagnosticSeconds
        CargoProfile = $step.profile
        CargoTargetDirectory = $targetDirectory
    }
    if ($CaptureCpuProfile) {
        $harnessArgs.CaptureCpuProfile = $true
    }

    try {
        & $harness @harnessArgs *> $stepLog
        if (-not $?) {
            throw "O harness retornou falha na etapa $($step.label)."
        }
        Get-Content -LiteralPath $stepLog | Tee-Object -FilePath $campaignLog -Append | Out-Host

        $manifestFiles = @(Get-ChildItem -LiteralPath $stepOutput -Filter "manifest.json" -File -Recurse)
        if ($manifestFiles.Count -ne 1) {
            throw "Esperado exatamente um manifesto em $stepOutput; encontrados: $($manifestFiles.Count)."
        }
        $manifestPath = $manifestFiles[0].FullName
        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        if ($manifest.exit_code -ne 0 -or $manifest.working_tree_dirty) {
            throw "Manifesto inválido ou campanha não concluída: $manifestPath"
        }
        if ($manifest.commit -ne $commit -or $manifest.rom_sha256 -ne $romHash) {
            throw "Commit ou SHA-256 da ROM divergiu durante a campanha."
        }
        if ($manifest.cargo_profile -ne $step.profile) {
            throw "Perfil registrado não corresponde à etapa $($step.label)."
        }

        $csvPath = Join-Path $manifestFiles[0].DirectoryName "outrun-a-b-c.csv"
        if (-not (Test-Path -LiteralPath $csvPath -PathType Leaf)) {
            throw "CSV não encontrado: $csvPath"
        }
        $csvRows = @(Import-Csv -LiteralPath $csvPath)
        if ($csvRows.Count -ne 3) {
            throw "A etapa $($step.label) deveria produzir exatamente três cenários; recebeu $($csvRows.Count)."
        }

        foreach ($row in $csvRows) {
            $allRows.Add([pscustomobject]@{
                sequence = $step.index
                run = $step.label
                profile = $step.profile
                scenario = $row.scenario
                frames = [long]$row.frames
                elapsed_ns = [long]$row.elapsed_ns
                throughput_fps = [double]::Parse($row.throughput_fps, [Globalization.CultureInfo]::InvariantCulture)
                sink_samples = [long]$row.sink_samples
                fm_address_writes = [long]$row.fm_address_writes
                fm_data_writes = [long]$row.fm_data_writes
                manifest = $manifestPath
                csv = $csvPath
            })
        }

        $runRecords.Add([pscustomobject]@{
            sequence = $step.index
            label = $step.label
            profile = $step.profile
            manifest = $manifestPath
            csv = $csvPath
            cargo_target_directory = $targetDirectory
            completed = $true
        })
    }
    catch {
        $failure = $_
        $runRecords.Add([pscustomobject]@{
            sequence = $step.index
            label = $step.label
            profile = $step.profile
            manifest = $null
            csv = $null
            cargo_target_directory = $targetDirectory
            completed = $false
            error = $_.Exception.Message
        })
        break
    }
}

$combinedCsv = Join-Path $runRoot "abba-results.csv"
$summaryCsv = Join-Path $runRoot "abba-summary.csv"
if ($allRows.Count -gt 0) {
    $allRows | Export-Csv -LiteralPath $combinedCsv -NoTypeInformation -Encoding UTF8
}

$summary = [System.Collections.Generic.List[object]]::new()
$scenarioNames = @("A-no-ym2413", "B-ym2413-clocked-muted", "C-ym2413-fm-mixed")
foreach ($scenario in $scenarioNames) {
    $means = @{}
    foreach ($profile in @("release", "release-llvm")) {
        $values = @($allRows | Where-Object { $_.scenario -eq $scenario -and $_.profile -eq $profile } | ForEach-Object { [double]$_.throughput_fps })
        if ($values.Count -gt 0) {
            $mean = ($values | Measure-Object -Average).Average
            $sumSquares = 0.0
            foreach ($value in $values) {
                $sumSquares += [Math]::Pow(($value - $mean), 2)
            }
            $stddev = if ($values.Count -gt 1) { [Math]::Sqrt($sumSquares / ($values.Count - 1)) } else { $null }
            $means[$profile] = $mean
            $summary.Add([pscustomobject]@{
                scenario = $scenario
                profile = $profile
                samples = $values.Count
                mean_fps = [Math]::Round($mean, 4)
                sample_stddev_fps = if ($null -eq $stddev) { $null } else { [Math]::Round($stddev, 4) }
                min_fps = [Math]::Round(($values | Measure-Object -Minimum).Minimum, 4)
                max_fps = [Math]::Round(($values | Measure-Object -Maximum).Maximum, 4)
                gain_vs_release_percent = $null
            })
        }
    }
    if ($means.ContainsKey("release") -and $means.ContainsKey("release-llvm")) {
        $gain = (($means["release-llvm"] / $means["release"]) - 1.0) * 100.0
        foreach ($entry in $summary | Where-Object { $_.scenario -eq $scenario -and $_.profile -eq "release-llvm" }) {
            $entry.gain_vs_release_percent = [Math]::Round($gain, 2)
        }
    }
}
if ($summary.Count -gt 0) {
    $summary | Export-Csv -LiteralPath $summaryCsv -NoTypeInformation -Encoding UTF8
}

$campaignManifest = [ordered]@{
    schema = "sun-perf-01-s1-abba-v1"
    captured_at_local = (Get-Date).ToString("o")
    repository = "leonardo201800478/SUN"
    commit = $commit
    branch = $branch
    working_tree_clean_at_start = $true
    rom_file = $rom.Name
    rom_sha256 = $romHash
    cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
    operating_system = (Get-CimInstance Win32_OperatingSystem).Caption
    rustc_version = (& rustc --version).Trim()
    cargo_version = (& cargo --version).Trim()
    target_triple = "x86_64-pc-windows-msvc"
    order = @("release", "release-llvm", "release-llvm", "release")
    warmup_seconds = $WarmupSeconds
    capture_seconds = $CaptureSeconds
    diagnostic_seconds = $DiagnosticSeconds
    cpu_profile_requested = [bool]$CaptureCpuProfile
    completed_runs = $runRecords.Count
    complete = ($runRecords.Count -eq 4 -and @($runRecords | Where-Object { -not $_.completed }).Count -eq 0)
    runs = @($runRecords)
    combined_csv = if (Test-Path -LiteralPath $combinedCsv) { $combinedCsv } else { $null }
    summary_csv = if (Test-Path -LiteralPath $summaryCsv) { $summaryCsv } else { $null }
    error = if ($null -ne $failure) { $failure.Exception.Message } else { $null }
}
$campaignManifestPath = Join-Path $runRoot "abba-manifest.json"
$campaignManifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $campaignManifestPath -Encoding UTF8

Write-Host ""
Write-Host "PERF-01/S1 ABBA artifacts:" -ForegroundColor Cyan
Write-Host "  Manifest: $campaignManifestPath"
if (Test-Path -LiteralPath $combinedCsv) { Write-Host "  Results:  $combinedCsv" }
if (Test-Path -LiteralPath $summaryCsv) { Write-Host "  Summary:  $summaryCsv" }
Write-Host "  Log:      $campaignLog"

if ($null -ne $failure) {
    throw "Campanha ABBA interrompida após $($runRecords.Count) etapa(s): $($failure.Exception.Message). Consulte $campaignManifestPath"
}
if (-not $campaignManifest.complete) {
    throw "Campanha ABBA incompleta. Consulte $campaignManifestPath"
}
