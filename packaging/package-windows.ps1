$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

# Keep in sync with CARGO_PACKAGER_VERSION in .github/workflows/package.yml.
$PackagerVersion = "0.11.8"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot
try {
    # No certificate, thumbprint, timestamp server, or signing command is configured.
    # The MSI is intentionally unsigned.
    $Target = "x86_64-pc-windows-msvc"

    cargo packager --version 2>$null | Out-Null
    if ($LASTEXITCODE -ne 0) {
        cargo install cargo-packager --version $PackagerVersion --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo install cargo-packager failed" }
    }

    cargo build -p oxide-ssh-desktop --release --target $Target --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

    # Packager.toml resolves binaries from target/package.
    $PackageDir = Join-Path $RepoRoot "target/package"
    New-Item -ItemType Directory -Force -Path $PackageDir | Out-Null
    Copy-Item (Join-Path $RepoRoot "target/$Target/release/oxide-ssh.exe") $PackageDir -Force

    cargo packager --release --target $Target --formats wix
    if ($LASTEXITCODE -ne 0) { throw "cargo packager failed" }
}
finally {
    Pop-Location
}
