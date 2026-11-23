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
    [string]$ThroughputCargoProfile = "release-llvm",
    [ValidateSet("release-llvm-debug")]
    [string]$ProfilingCargoProfile = "release-llvm-debug",
    [string]$CargoTargetDirectory = "",
    # Retained as a compatibility parameter. Heap tracing is intentionally removed.
    [switch]$SkipHeapProfile
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$baselineScript = Join-Path $PSScriptRoot "perf-baseline-s0.ps1"
if (-not (Test-Path -LiteralPath $baselineScript -PathType Leaf)) {
    throw "Harness S0 agregado não encontrado: $baselineScript"
}

Write-Warning "O perfil antigo WPR Heap foi removido: a campanha agora usa contadores agregados em memória e gera somente CSV/JSON. Nenhum ETL/heap trace será criado."

$arguments = @{
    RomPath = $RomPath
    WarmupSeconds = $WarmupSeconds
    CaptureSeconds = $CaptureSeconds
    DiagnosticSeconds = $DiagnosticSeconds
    CargoProfile = $ThroughputCargoProfile
}
if (-not [string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $arguments.OutputDirectory = $OutputDirectory
}
if (-not [string]::IsNullOrWhiteSpace($CargoTargetDirectory)) {
    $arguments.CargoTargetDirectory = $CargoTargetDirectory
}

& $baselineScript @arguments
if ($LASTEXITCODE -ne 0) {
    throw "A campanha S0 agregada falhou."
}
