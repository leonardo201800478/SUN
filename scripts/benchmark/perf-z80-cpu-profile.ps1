[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidateScript({ Test-Path -LiteralPath $_ -PathType Leaf })]
    [string]$RomPath,
    [string]$OutputDirectory = "",
    [ValidateRange(1, 600)]
    [int]$WarmupSeconds = 5,
    [ValidateRange(1, 1800)]
    [int]$CaptureSeconds = 30
)
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") { throw "Windows x64 is required." }
foreach ($tool in @("git", "cargo", "rustc", "wpr")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool not found in PATH." }
}
$root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$rom = Get-Item -LiteralPath (Resolve-Path -LiteralPath $RomPath).Path
if ($rom.Name -notmatch "(?i)outrun") { throw "OutRun is the required workload." }
$dirty = -not [string]::IsNullOrWhiteSpace((& git -C $root status --porcelain))
if ($LASTEXITCODE -ne 0) { throw "Could not inspect Git working tree." }
if ($dirty) { throw "Working tree must be clean." }
$commit = (& git -C $root rev-parse HEAD).Trim()
$branch = (& git -C $root branch --show-current).Trim()
$romHash = (Get-FileHash -LiteralPath $rom.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "SUN-Benchmarks\perf-01-z80"
}
$out = Join-Path ([System.IO.Path]::GetFullPath($OutputDirectory)) (Get-Date -Format "yyyyMMdd-HHmmss")
New-Item -ItemType Directory -Path $out -Force | Out-Null
$env:CARGO_TARGET_DIR = Join-Path $out "cargo-target"
New-Item -ItemType Directory -Path $env:CARGO_TARGET_DIR -Force | Out-Null
$env:SEGA_PERF_ROM_PATH = $rom.FullName
$env:SEGA_PERF_PROFILE_WARMUP_SECS = [string]$WarmupSeconds
$env:SEGA_PERF_PROFILE_CAPTURE_SECS = [string]$CaptureSeconds

$buildLog = Join-Path $out "build.log"
Push-Location $root
try {
    & cargo test --profile release-llvm-debug --locked --target x86_64-pc-windows-msvc -p sega-sms-runtime --test perf_z80_profile --no-run -vv 2>&1 | Tee-Object -FilePath $buildLog | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "Profile harness build failed; see $buildLog" }
}
finally { Pop-Location }

$results = @()
$wprActive = $false
try {
    foreach ($scenario in @(@{Key="A";Name="A-no-ym2413"}, @{Key="B";Name="B-ym2413-clocked-muted"}, @{Key="C";Name="C-ym2413-fm-mixed"})) {
        $env:SEGA_PERF_PROFILE_SCENARIO = $scenario.Key
        $ready = Join-Path $out "$($scenario.Key).ready"
        $go = Join-Path $out "$($scenario.Key).go"
        $etl = Join-Path $out "cpu-profile-$($scenario.Key).etl"
        $stdout = Join-Path $out "profile-$($scenario.Key).stdout.log"
        $stderr = Join-Path $out "profile-$($scenario.Key).stderr.log"
        Remove-Item -LiteralPath $ready, $go -Force -ErrorAction SilentlyContinue
        $env:SEGA_PERF_PROFILE_READY_FILE = $ready
        $env:SEGA_PERF_PROFILE_GO_FILE = $go
        $args = @("test", "--profile", "release-llvm-debug", "--locked", "--target", "x86_64-pc-windows-msvc", "-p", "sega-sms-runtime", "--test", "perf_z80_profile", "outrun_single_scenario_cpu_profile", "--", "--ignored", "--nocapture")
        $cargo = (Get-Command cargo).Source
        $process = Start-Process -FilePath $cargo -ArgumentList $args -WorkingDirectory $root -PassThru -NoNewWindow -RedirectStandardOutput $stdout -RedirectStandardError $stderr
        $deadline = (Get-Date).AddMinutes(3)
        while (-not (Test-Path -LiteralPath $ready)) {
            if ($process.HasExited) { throw "Scenario $($scenario.Key) failed before ETW warmup gate. See $stdout and $stderr" }
            if ((Get-Date) -gt $deadline) { throw "Timed out waiting for scenario $($scenario.Key) warmup." }
            Start-Sleep -Milliseconds 50
        }
        & wpr -start CPU -filemode
        if ($LASTEXITCODE -ne 0) { throw "WPR failed to start. Run elevated PowerShell." }
        $wprActive = $true
        Set-Content -LiteralPath $go -Value "capture" -NoNewline
        $process.WaitForExit()
        & wpr -stop $etl
        $stopCode = $LASTEXITCODE
        $wprActive = $false
        if ($process.ExitCode -ne 0) { throw "Scenario $($scenario.Key) failed; see $stdout and $stderr" }
        if ($stopCode -ne 0 -or -not (Test-Path -LiteralPath $etl -PathType Leaf)) { throw "ETW trace missing for scenario $($scenario.Key)." }
        $results += [ordered]@{scenario=$scenario.Name; etl=[System.IO.Path]::GetFileName($etl); stdout=[System.IO.Path]::GetFileName($stdout); stderr=[System.IO.Path]::GetFileName($stderr)}
    }
}
finally {
    if ($wprActive) { & wpr -stop (Join-Path $out "cpu-profile-incomplete.etl") }
}

$manifest = [ordered]@{
    schema = "sun-perf-01-z80-cpu-profile-v1"
    captured_at = (Get-Date).ToString("o")
    repository = "leonardo201800478/SUN"
    commit = $commit
    branch = $branch
    rom_file = $rom.Name
    rom_sha256 = $romHash
    region = "japan"
    vdp = "sms1"
    target = "x86_64-pc-windows-msvc"
    profile = "release-llvm-debug"
    rustc = (& rustc --version --verbose | Out-String).Trim()
    cargo = (& cargo --version).Trim()
    cpu = (Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)
    os = (Get-CimInstance Win32_OperatingSystem).Caption
    warmup_seconds = $WarmupSeconds
    capture_seconds = $CaptureSeconds
    diagnostics = "disabled"
    pacing = "disabled"
    presentation = "disabled"
    audio_sink = "native headless mixer/resampler; CPAL excluded"
    scenarios = $results
}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $out "manifest.json") -Encoding UTF8
Write-Host "Z80 ETW profile artifacts: $out" -ForegroundColor Green
