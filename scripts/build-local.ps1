param([switch]$Run)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $zigBin = Join-Path $root 'target/tools/zig-0.17.1/zig-x86_64-windows-0.17.1'
    if (Test-Path (Join-Path $zigBin 'zig.exe')) { $env:PATH = "$zigBin;$env:PATH" }
    foreach ($tool in @('rustc', 'cargo', 'zig', 'git', 'cl')) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            throw "Missing '$tool'. Install Rust, Zig 0.17.1, Git for Windows, and Visual Studio C++ Build Tools; run from a Visual Studio Developer PowerShell."
        }
    }
    $gitExe = (Get-Command git).Source
    $gitBash = Join-Path (Split-Path (Split-Path $gitExe -Parent) -Parent) 'bin/bash.exe'
    if (-not (Test-Path $gitBash)) { throw "Git for Windows Bash not found at '$gitBash'." }

    $zigVersion = (& zig version).Trim()
    if ($LASTEXITCODE -ne 0 -or $zigVersion -ne '0.17.1') {
        throw "Expected Zig 0.17.1; found '$zigVersion'."
    }

    $hostMatch = & rustc -vV | Select-String '^host:\s*(\S+)$'
    if (-not $hostMatch) { throw 'Could not determine the Rust host target.' }
    $target = $hostMatch.Matches[0].Groups[1].Value
    if ($target -notin @('x86_64-pc-windows-msvc', 'aarch64-pc-windows-msvc')) {
        throw "Unsupported Windows host target '$target'."
    }

    $nativeRoot = Join-Path $root "target/libghostty-vt/$target"
    $libraryDir = Join-Path $nativeRoot 'stage/usr/lib'
    $library = Join-Path $libraryDir 'ghostty-vt-static.lib'
    if (-not (Test-Path $library) -or -not (Test-Path (Join-Path $nativeRoot 'install-root.txt'))) {
        $buildScript = Join-Path $root 'scripts/build-libghostty-vt.sh'
        $normalizedScript = Join-Path $root "scripts/.build-libghostty-vt-$PID.sh"
        try {
            $scriptText = [System.IO.File]::ReadAllText($buildScript).Replace("`r`n", "`n")
            [System.IO.File]::WriteAllText($normalizedScript, $scriptText, [System.Text.UTF8Encoding]::new($false))
            & $gitBash $normalizedScript $target
            if ($LASTEXITCODE -ne 0) { throw 'Building libghostty-vt failed.' }
        } finally {
            Remove-Item $normalizedScript -ErrorAction SilentlyContinue
        }
    }

    $env:LIBGHOSTTY_VT_LIB_DIR = $libraryDir
    if ($Run) {
        & cargo run -p oxide-ssh-desktop --locked
    } else {
        & cargo build -p oxide-ssh-desktop --locked
    }
    if ($LASTEXITCODE -ne 0) { throw 'Cargo build failed.' }
} finally {
    Pop-Location
}
