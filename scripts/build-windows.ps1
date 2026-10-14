param(
    [switch]$Installer,
    [switch]$Test
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Cargo was not found. Install Rust with rustup.rs, then open a new PowerShell window.'
    }

    cargo fmt --all
    if ($LASTEXITCODE -ne 0) { throw 'cargo fmt failed.' }

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed after formatting.' }

    if ($Test) {
        cargo test --workspace
        if ($LASTEXITCODE -ne 0) { throw 'Privora core tests failed.' }
    }

    cargo build --release -p privora-desktop
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }

    $exe = Join-Path $root 'target\release\privora-desktop.exe'
    if (-not (Test-Path $exe)) { throw "Expected executable not found: $exe" }
    Write-Host "Built: $exe" -ForegroundColor Green

    if ($Installer) {
        $iscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue
        if ($iscc) {
            $isccPath = $iscc.Source
        } else {
            $candidates = @()
            if (${env:ProgramFiles(x86)}) { $candidates += (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe') }
            if ($env:LOCALAPPDATA) { $candidates += (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe') }
            if ($env:ProgramFiles) { $candidates += (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe') }
            $isccPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
        }
        if (-not $isccPath) {
            throw 'Inno Setup compiler ISCC.exe was not found. Install Inno Setup 6, then rerun with -Installer.'
        }
        New-Item -ItemType Directory -Force -Path (Join-Path $root 'dist') | Out-Null
        Push-Location (Join-Path $root 'packaging\windows')
        try {
            & $isccPath 'Privora.iss'
            if ($LASTEXITCODE -ne 0) { throw 'Inno Setup failed to create the installer.' }
        } finally {
            Pop-Location
        }
        Write-Host "Installer output: $(Join-Path $root 'dist\Privora-Setup-0.4.0-x64.exe')" -ForegroundColor Green
    }
} finally {
    Pop-Location
}
