param(
    [switch]$Check
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$manifest = Get-Content -Raw (Join-Path $root Cargo.toml)
$workspaceVersion = [regex]::Match($manifest, '(?ms)^\[workspace\.package\].*?^version = "([^"]+)"').Groups[1].Value
if (-not $workspaceVersion) {
    throw "workspace.package.version not found in Cargo.toml"
}

$packagerPath = Join-Path $root Packager.toml
$packager = Get-Content -Raw $packagerPath
$packagerVersion = [regex]::Match($packager, '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $packagerVersion) {
    throw "version not found in Packager.toml"
}

if ($Check) {
    if ($workspaceVersion -ne $packagerVersion) {
        throw "Packager.toml version '$packagerVersion' does not match workspace version '$workspaceVersion'"
    }
    $lock = (Get-Content -Raw (Join-Path $root Cargo.lock)) -replace "`r`n", "`n"
    foreach ($package in @('oxide-ssh-app', 'oxide-ssh-core', 'oxide-ssh-terminal', 'oxide-ssh-desktop')) {
        if ($lock -notmatch "(?ms)^\[\[package\]\]\nname = `"$package`"\nversion = `"$([regex]::Escape($workspaceVersion))`"") {
            throw "Cargo.lock version for $package does not match workspace version '$workspaceVersion'"
        }
    }
    Write-Output "Workspace and package versions are $workspaceVersion"
    exit
}

$updated = [regex]::Replace($packager, '(?m)^version = "[^"]+"', "version = `"$workspaceVersion`"", 1)
if ($updated -ne $packager) {
    Set-Content -Path $packagerPath -Value $updated -NoNewline
}
& "$PSScriptRoot/sync-version.ps1" -Check
