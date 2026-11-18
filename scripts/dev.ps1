# yob developer commands from PowerShell: ./scripts/dev.ps1 <command>
# Forwards to dev.sh through Git Bash. All logic lives in dev.sh; add new
# commands there, not here.
#
# Plain `bash` in PowerShell starts WSL, which can't see the Windows Rust and
# Node installs, so this finds Git for Windows' bash explicitly.

$candidates = @()
$git = Get-Command git -ErrorAction SilentlyContinue
if ($git) {
    # git.exe lives in <Git>\cmd or <Git>\mingw64\bin; bash is in <Git>\bin.
    $dir = Split-Path $git.Source
    $candidates += (Join-Path (Split-Path $dir) "bin\bash.exe")
    $candidates += (Join-Path (Split-Path (Split-Path $dir)) "bin\bash.exe")
}
$candidates += "C:\Program Files\Git\bin\bash.exe"

$bash = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $bash) {
    Write-Error "Git Bash not found. Install Git for Windows: winget install Git.Git"
    exit 1
}

& $bash "$PSScriptRoot/dev.sh" @args
exit $LASTEXITCODE
