[CmdletBinding()]
param(
    [string]$RepositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path,
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "Programs\codex-local\bin"),
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-Checked {
    param(
        [string]$Program,
        [string[]]$Arguments
    )

    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Program exited with code $LASTEXITCODE."
    }
}

function Resolve-PythonInterpreter {
    $python = Get-Command python.exe -All -ErrorAction SilentlyContinue |
        Where-Object { $_.Source -notlike "*\Microsoft\WindowsApps\*" } |
        Select-Object -First 1
    if ($null -ne $python) {
        return $python.Source
    }

    $launcher = Get-Command py.exe -ErrorAction SilentlyContinue
    if ($null -ne $launcher) {
        $resolved = & $launcher.Source -3 -c "import sys; print(sys.executable)"
        if ($LASTEXITCODE -eq 0 -and (Test-Path -LiteralPath $resolved -PathType Leaf)) {
            return $resolved
        }
    }

    throw "A real Python 3 interpreter is required; the Windows Store app alias is not sufficient."
}

$RepositoryRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$CargoRoot = Join-Path $RepositoryRoot "codex-rs"
$ReleaseDir = Join-Path $CargoRoot "target\release"
$CodexSource = Join-Path $ReleaseDir "codex.exe"
$HostSource = Join-Path $ReleaseDir "codex-code-mode-host.exe"

if (-not $SkipBuild) {
    $python = Resolve-PythonInterpreter
    $env:PYTHON = $python
    $env:CODEX_REPO_ROOT = $RepositoryRoot
    $env:PYTHONPATH = Join-Path $RepositoryRoot "scripts"
    $v8Json = & $python -c "import json; from codex_package.targets import TARGET_SPECS, default_target; from codex_package.v8 import resolve_codex_v8_cargo_env; print(json.dumps(resolve_codex_v8_cargo_env(TARGET_SPECS[default_target()])))"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to resolve the checksum-verified Codex V8 artifacts."
    }
    $v8Environment = $v8Json | ConvertFrom-Json
    $env:RUSTY_V8_ARCHIVE = $v8Environment.RUSTY_V8_ARCHIVE
    $env:RUSTY_V8_SRC_BINDING_PATH = $v8Environment.RUSTY_V8_SRC_BINDING_PATH

    Push-Location $CargoRoot
    try {
        Invoke-Checked cargo @("build", "--release", "-p", "codex-cli", "--bin", "codex")
        Invoke-Checked cargo @(
            "build",
            "--release",
            "-p",
            "codex-code-mode-host",
            "--bin",
            "codex-code-mode-host"
        )
    } finally {
        Pop-Location
    }
}

$requiredSources = @($CodexSource, $HostSource)
foreach ($source in $requiredSources) {
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Required local installation binary is missing: $source"
    }
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
$installed = @(
    @{ Source = $CodexSource; Destination = (Join-Path $InstallDir "codex-local.exe") },
    @{ Source = $HostSource; Destination = (Join-Path $InstallDir "codex-code-mode-host.exe") }
)
foreach ($binary in $installed) {
    $sourceHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary.Source).Hash
    $installedHash = if (Test-Path -LiteralPath $binary.Destination -PathType Leaf) {
        (Get-FileHash -Algorithm SHA256 -LiteralPath $binary.Destination).Hash
    } else {
        $null
    }
    if ($sourceHash -ne $installedHash) {
        try {
            Copy-Item -LiteralPath $binary.Source -Destination $binary.Destination -Force
        } catch {
            throw "Could not update $($binary.Destination). Exit any running codex-local session and retry. $($_.Exception.Message)"
        }
        $installedHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $binary.Destination).Hash
    }
    if ($sourceHash -ne $installedHash) {
        throw "Installed binary hash mismatch: $($binary.Destination)"
    }
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @($userPath -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
$hasInstallDir = $pathEntries | Where-Object {
    $_.TrimEnd("\") -ieq $InstallDir.TrimEnd("\")
}
if (-not $hasInstallDir) {
    [Environment]::SetEnvironmentVariable("Path", (($pathEntries + $InstallDir) -join ";"), "User")
}

Write-Host "Installed codex-local and codex-code-mode-host to $InstallDir"
Write-Host "Open a new terminal before invoking codex-local by name."
