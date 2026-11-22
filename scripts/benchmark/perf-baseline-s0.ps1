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

    [ValidateSet("release", "release-llvm", "release-llvm-debug")]
    [string]$CargoProfile = "release",

    # Optional shared Cargo target directory for repeated benchmark campaigns.
    [string]$CargoTargetDirectory = "",

    [switch]$CaptureCpuProfile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($env:OS -ne "Windows_NT") {
    throw "A campanha PERF-01/S0 exige Windows x64."
}
if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
    throw "Git não foi encontrado no PATH."
}
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo não foi encontrado no PATH."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$rom = Get-Item -LiteralPath (Resolve-Path -LiteralPath $RomPath).Path
if ($rom.Name -notmatch "(?i)outrun") {
    throw "OutRun é o benchmark principal da PERF-01. ROM informada: $($rom.Name)"
}

$romHash = (Get-FileHash -LiteralPath $rom.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
$commit = (& git -C $repoRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) {
    throw "Não foi possível identificar o commit do código."
}
$branch = (& git -C $repoRoot branch --show-current).Trim()
$dirty = -not [string]::IsNullOrWhiteSpace((& git -C $repoRoot status --porcelain))
if ($dirty) {
    throw "O working tree precisa estar limpo para associar os resultados a um commit reproduzível."
}
$rustcVersion = (& rustc --version --verbose 2>&1 | Out-String).Trim()
$cargoVersion = (& cargo --version 2>&1 | Out-String).Trim()
$cpuName = (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
$osCaption = (Get-CimInstance Win32_OperatingSystem).Caption
$cargoConfig = @()
foreach ($relativePath in @(".cargo/config.toml", ".cargo/config")) {
    $configPath = Join-Path $repoRoot $relativePath
    if (Test-Path -LiteralPath $configPath -PathType Leaf) {
        $cargoConfig += [ordered]@{
            path = $relativePath
            content = Get-Content -LiteralPath $configPath -Raw
        }
    }
}

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $documents = [Environment]::GetFolderPath("MyDocuments")
    $OutputDirectory = Join-Path $documents "SUN-Benchmarks\perf-01-s0"
}
$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$outputRoot = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) $stamp
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null

$csvPath = Join-Path $outputRoot "outrun-a-b-c.csv"
$logPath = Join-Path $outputRoot "benchmark.log"
$manifestPath = Join-Path $outputRoot "manifest.json"
$etlPath = Join-Path $outputRoot "cpu-profile.etl"
$buildLogPath = Join-Path $outputRoot "build.log"

$envNames = @(
    "SEGA_PERF_ROM_PATH",
    "SEGA_PERF_WARMUP_SECS",
    "SEGA_PERF_CAPTURE_SECS",
    "SEGA_PERF_DIAGNOSTIC_SECS",
    "SEGA_PERF_OUTPUT_CSV",
    "CARGO_TARGET_DIR"
)
$previousEnv = @{}
foreach ($name in $envNames) {
    $previousEnv[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

if ([string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
    $cargoTargetDir = Join-Path $outputRoot "cargo-target"
}
else {
    $cargoTargetDir = [System.IO.Path]::GetFullPath($CargoTargetDirectory)
}
New-Item -ItemType Directory -Path $cargoTargetDir -Force | Out-Null
$wprStarted = $false
$exitCode = 1
try {
    if ($CaptureCpuProfile -and $CargoProfile -ne "release-llvm-debug") {
        throw "Use -CargoProfile release-llvm-debug with -CaptureCpuProfile to retain symbols for function-level attribution."
    }

    $env:SEGA_PERF_ROM_PATH = $rom.FullName
    $env:SEGA_PERF_WARMUP_SECS = [string]$WarmupSeconds
    $env:SEGA_PERF_CAPTURE_SECS = [string]$CaptureSeconds
    $env:SEGA_PERF_DIAGNOSTIC_SECS = [string]$DiagnosticSeconds
    $env:SEGA_PERF_OUTPUT_CSV = $csvPath
    $env:CARGO_TARGET_DIR = $cargoTargetDir

    Push-Location $repoRoot
    try {
        # Compile the exact test target before WPR starts. Otherwise a fresh
        # Cargo target directory makes rustc/LLVM/linker samples contaminate
        # the emulator CPU profile.
        & cargo test --profile $CargoProfile --locked --target x86_64-pc-windows-msvc -vv -p sega-sms-runtime --test perf_baseline --no-run 2>&1 |
            Tee-Object -FilePath $buildLogPath |
            Out-Host
        $buildExitCode = $LASTEXITCODE
        if ($buildExitCode -ne 0) {
            throw "A compilação prévia do benchmark falhou. Consulte $buildLogPath"
        }

        if ($CaptureCpuProfile) {
            if (-not (Get-Command wpr -ErrorAction SilentlyContinue)) {
                throw "Windows Performance Recorder (wpr.exe) não foi encontrado."
            }
            & wpr -start CPU -filemode
            if ($LASTEXITCODE -ne 0) {
                throw "WPR não iniciou o perfil CPU. Execute o PowerShell elevado e tente novamente."
            }
            $wprStarted = $true
        }

        # The target is already built; this invocation runs the benchmark
        # without compiling Rust code inside the ETW sampling window.
        & cargo test --profile $CargoProfile --locked --target x86_64-pc-windows-msvc -vv -p sega-sms-runtime --test perf_baseline outrun_a_b_c_uncapped_baseline -- --ignored --nocapture 2>&1 |
            Tee-Object -FilePath $logPath |
            Out-Host
        $exitCode = $LASTEXITCODE
    }
    finally {
        Pop-Location
    }
}
finally {
    if ($wprStarted) {
        & wpr -stop $etlPath
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "WPR não conseguiu finalizar o arquivo ETL. Verifique a sessão de gravação."
        }
    }

    foreach ($name in $envNames) {
        [Environment]::SetEnvironmentVariable($name, $previousEnv[$name], "Process")
    }
}

$manifest = [ordered]@{
    schema = "sun-perf-01-s0-v1"
    captured_at_local = (Get-Date).ToString("o")
    repository = "leonardo201800478/SUN"
    commit = $commit
    branch = $branch
    working_tree_dirty = $dirty
    rom_file = $rom.Name
    rom_sha256 = $romHash
    region = "japan"
    vdp = "sms1"
    scenarios = @(
        "A-no-ym2413",
        "B-ym2413-clocked-muted",
        "C-ym2413-fm-mixed"
    )
    warmup_seconds = $WarmupSeconds
    capture_seconds = $CaptureSeconds
    diagnostic_seconds = $DiagnosticSeconds
    frame_pacing = "disabled"
    video_presentation = "disabled"
    audio_sink = "headless native mixer/resampler; CPAL measured separately"
    cpu = $cpuName
    operating_system = $osCaption
    cargo_version = $cargoVersion
    rustc_version = $rustcVersion
    target_triple = "x86_64-pc-windows-msvc"
    cargo_profile = $CargoProfile
    cargo_target_dir = $cargoTargetDir
    cargo_config = $cargoConfig
    compiler_environment = [ordered]@{
        RUSTFLAGS = $env:RUSTFLAGS
        CARGO_ENCODED_RUSTFLAGS = $env:CARGO_ENCODED_RUSTFLAGS
        CARGO_PROFILE_RELEASE_OPT_LEVEL = $env:CARGO_PROFILE_RELEASE_OPT_LEVEL
        CARGO_PROFILE_RELEASE_LTO = $env:CARGO_PROFILE_RELEASE_LTO
        CARGO_PROFILE_RELEASE_CODEGEN_UNITS = $env:CARGO_PROFILE_RELEASE_CODEGEN_UNITS
        CARGO_PROFILE_RELEASE_STRIP = $env:CARGO_PROFILE_RELEASE_STRIP
        CARGO_PROFILE_SELECTED_OPT_LEVEL = [Environment]::GetEnvironmentVariable(("CARGO_PROFILE_" + $CargoProfile.ToUpperInvariant().Replace("-", "_") + "_OPT_LEVEL"), "Process")
        CARGO_PROFILE_SELECTED_LTO = [Environment]::GetEnvironmentVariable(("CARGO_PROFILE_" + $CargoProfile.ToUpperInvariant().Replace("-", "_") + "_LTO"), "Process")
        CARGO_PROFILE_SELECTED_CODEGEN_UNITS = [Environment]::GetEnvironmentVariable(("CARGO_PROFILE_" + $CargoProfile.ToUpperInvariant().Replace("-", "_") + "_CODEGEN_UNITS"), "Process")
    }
    cpu_profile_requested = [bool]$CaptureCpuProfile
    cpu_profile_path = if ($CaptureCpuProfile) { $etlPath } else { $null }
    cpu_profile_symbolized = ($CaptureCpuProfile -and $CargoProfile -eq "release-llvm-debug")
    throughput_profile_is_diagnostic = ($CargoProfile -eq "release-llvm-debug")
    build_log_path = $buildLogPath
    exit_code = $exitCode
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

Write-Host ""
Write-Host "PERF-01/S0 artifacts:" -ForegroundColor Cyan
Write-Host "  Manifest: $manifestPath"
Write-Host "  CSV:      $csvPath"
Write-Host "  Build:    $buildLogPath"
Write-Host "  Log:      $logPath"
if ($CaptureCpuProfile) {
    Write-Host "  CPU ETL:  $etlPath"
}
if ($exitCode -ne 0) {
    throw "A campanha de benchmark falhou. Consulte $logPath"
}
