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
    [int]$DiagnosticSeconds = 10,
    [ValidateSet("release", "release-llvm", "release-llvm-debug")]
    [string]$CargoProfile = "release-llvm-debug",
    [string]$CargoTargetDirectory = "",
    [switch]$SkipHeapProfile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") { throw "A campanha S0 System Profile exige Windows x64." }
foreach ($command in @("git", "cargo", "rustc", "wpr")) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "$command não foi encontrado no PATH." }
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Execute o PowerShell como Administrador para habilitar WPR e Heap tracing."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$rom = Get-Item -LiteralPath (Resolve-Path -LiteralPath $RomPath).Path
if ($rom.Name -notmatch "(?i)outrun") { throw "O benchmark S0 primário exige OutRun SMS. ROM informada: $($rom.Name)" }
$romHash = (Get-FileHash -LiteralPath $rom.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw "Não foi possível identificar o commit." }
$branch = (& git -C $repoRoot branch --show-current).Trim()
if (-not [string]::IsNullOrWhiteSpace((& git -C $repoRoot status --porcelain))) {
    throw "O working tree deve estar limpo para uma campanha reproduzível."
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SUN-Benchmarks\S0-System-Profile"
}
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outputRoot = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) $stamp
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
if ([string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
    $cargoTarget = Join-Path $outputRoot "cargo-target"
} else {
    $cargoTarget = [System.IO.Path]::GetFullPath($CargoTargetDirectory)
}
New-Item -ItemType Directory -Path $cargoTarget -Force | Out-Null

$baselineScript = Join-Path $PSScriptRoot "perf-baseline-s0.ps1"
if (-not (Test-Path -LiteralPath $baselineScript -PathType Leaf)) { throw "Harness S0 existente não encontrado: $baselineScript" }

# Pass 1: authoritative throughput run. No ETW, heap tracing or profiler is active.
$baselineArgs = @{
    RomPath = $rom.FullName
    OutputDirectory = (Join-Path $outputRoot "baseline")
    WarmupSeconds = $WarmupSeconds
    CaptureSeconds = $CaptureSeconds
    DiagnosticSeconds = $DiagnosticSeconds
    CargoProfile = $CargoProfile
    CargoTargetDirectory = $cargoTarget
}
& $baselineScript @baselineArgs
if ($LASTEXITCODE -ne 0) { throw "A baseline S0 sem profiler falhou. Consulte a pasta baseline." }

# Build before ETW starts. The profiled execution is separate and its FPS is not
# the authoritative throughput result.
$buildLog = Join-Path $outputRoot "profile-build.log"
Push-Location $repoRoot
try {
    & cargo test --profile $CargoProfile --locked --target x86_64-pc-windows-msvc -p sega-sms-runtime --test perf_baseline --no-run 2>&1 |
        Tee-Object -FilePath $buildLog |
        Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Build do harness de profiling falhou." }
}
finally { Pop-Location }

$profileDirectory = Join-Path (Join-Path $cargoTarget "x86_64-pc-windows-msvc") $CargoProfile
$testBinary = Get-ChildItem -LiteralPath (Join-Path $profileDirectory "deps") -Filter "perf_baseline-*.exe" -File |
    Sort-Object LastWriteTimeUtc -Descending |
    Select-Object -First 1
if ($null -eq $testBinary) { throw "Não foi possível localizar o executável perf_baseline em $profileDirectory\deps." }

$profileLog = Join-Path $outputRoot "profiled-run.log"
$profileCsv = Join-Path $outputRoot "profiled-throughput.csv"
$diagnosticsCsv = Join-Path $outputRoot "subsystem-diagnostics.csv"
$etlPath = Join-Path $outputRoot "cpu-and-heap-profile.etl"
$heapTracingEnabled = $false
$wprStarted = $false
$profileExitCode = 1
$envNames = @("SEGA_PERF_ROM_PATH", "SEGA_PERF_WARMUP_SECS", "SEGA_PERF_CAPTURE_SECS", "SEGA_PERF_DIAGNOSTIC_SECS", "SEGA_PERF_OUTPUT_CSV")
$previousEnv = @{}
foreach ($name in $envNames) { $previousEnv[$name] = [Environment]::GetEnvironmentVariable($name, "Process") }

try {
    $env:SEGA_PERF_ROM_PATH = $rom.FullName
    $env:SEGA_PERF_WARMUP_SECS = [string]$WarmupSeconds
    $env:SEGA_PERF_CAPTURE_SECS = [string]$CaptureSeconds
    $env:SEGA_PERF_DIAGNOSTIC_SECS = [string]$DiagnosticSeconds
    $env:SEGA_PERF_OUTPUT_CSV = $profileCsv

    if (-not $SkipHeapProfile) {
        & wpr -HeapTracingConfig $testBinary.Name enable
        if ($LASTEXITCODE -ne 0) { throw "WPR não habilitou heap tracing para $($testBinary.Name)." }
        $heapTracingEnabled = $true
        & wpr -start CPU -start Heap -filemode
    } else {
        & wpr -start CPU -filemode
    }
    if ($LASTEXITCODE -ne 0) { throw "WPR não iniciou a sessão CPU/Heap." }
    $wprStarted = $true
    & $testBinary.FullName outrun_a_b_c_uncapped_baseline --ignored --nocapture 2>&1 |
        Tee-Object -FilePath $profileLog |
        Out-Host
    $profileExitCode = $LASTEXITCODE
}
finally {
    if ($wprStarted) {
        & wpr -stop $etlPath
        if ($LASTEXITCODE -ne 0) { Write-Warning "WPR não finalizou o ETL corretamente. Verifique wpr -status." }
    }
    if ($heapTracingEnabled) {
        & wpr -HeapTracingConfig $testBinary.Name disable
        if ($LASTEXITCODE -ne 0) { Write-Warning "Não foi possível desabilitar heap tracing de $($testBinary.Name). Desabilite manualmente." }
    }
    foreach ($name in $envNames) { [Environment]::SetEnvironmentVariable($name, $previousEnv[$name], "Process") }
}
if ($profileExitCode -ne 0) { throw "A passagem instrumentada falhou. A baseline não instrumentada permanece preservada." }

# Export existing worker diagnostic records as a machine-readable CSV.
$diagnosticRows = @()
foreach ($line in Get-Content -LiteralPath $profileLog) {
    if ($line -notmatch "SUN_PERF_DIAGNOSTICS") { continue }
    $row = [ordered]@{}
    foreach ($match in [regex]::Matches($line, '([A-Za-z0-9_]+)=([^\s]+)')) {
        $row[$match.Groups[1].Value] = $match.Groups[2].Value
    }
    if ($row.Count -gt 0) { $diagnosticRows += [pscustomobject]$row }
}
if ($diagnosticRows.Count -gt 0) {
    $diagnosticRows | Export-Csv -LiteralPath $diagnosticsCsv -NoTypeInformation -Encoding UTF8
} else {
    Write-Warning "Nenhuma linha SUN_PERF_DIAGNOSTICS foi encontrada; confira o log instrumentado."
}

$manifest = [ordered]@{
    schema = "sun-s0-system-profile-v1"
    captured_at_local = (Get-Date).ToString("o")
    repository = "leonardo201800478/SUN"
    commit = $commit
    branch = $branch
    working_tree_clean = $true
    rom_file = $rom.Name
    rom_sha256 = $romHash
    target = "x86_64-pc-windows-msvc"
    cargo_profile = $CargoProfile
    cargo_version = (& cargo --version).Trim()
    rustc_version = (& rustc --version --verbose | Out-String).Trim()
    cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
    operating_system = (Get-CimInstance Win32_OperatingSystem).Caption
    warmup_seconds = $WarmupSeconds
    throughput_capture_seconds = $CaptureSeconds
    diagnostic_capture_seconds = $DiagnosticSeconds
    throughput_pass = "unprofiled; authoritative"
    profiling_pass = "separate process execution; diagnostic only"
    profilers = if ($SkipHeapProfile) { @("WPR CPU") } else { @("WPR CPU", "WPR Heap") }
    test_executable = $testBinary.Name
    baseline_directory = Join-Path $outputRoot "baseline"
    cpu_heap_etl = $etlPath
    profiled_log = $profileLog
    profiled_throughput_csv = $profileCsv
    subsystem_diagnostics_csv = if (Test-Path -LiteralPath $diagnosticsCsv) { $diagnosticsCsv } else { $null }
    profile_exit_code = $profileExitCode
    heap_profile_included = (-not $SkipHeapProfile)
    limitations = @(
        "WPR CPU samples require WPA and matching Rust PDB symbols for function-level attribution.",
        "WPR Heap covers Windows process heap allocations; custom allocators or non-heap mappings may require separate instrumentation.",
        "Current SMS worker diagnostics expose frame, YM2413, video publication and audio prepare/submit aggregates; they do not yet provide exclusive timers for every hardware subsystem.",
        "This headless run does not measure physical CPAL, Slint composition, WGPU upload or GPU execution."
    )
}
$manifestPath = Join-Path $outputRoot "manifest.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

Write-Host ""
Write-Host "SUN S0 System Profile artifacts:" -ForegroundColor Cyan
Write-Host "  Manifest:     $manifestPath"
Write-Host "  Baseline:     $($manifest.baseline_directory)"
Write-Host "  CPU/Heap ETL: $etlPath"
Write-Host "  Diagnostics:  $diagnosticsCsv"
Write-Host "  Profile log:  $profileLog"
