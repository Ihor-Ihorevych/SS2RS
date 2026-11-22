# Run ss2-app with the puffin profiler built in and switched on (docs/PROFILING.md).
# Extra arguments go to the app. Set SS2_PROFILE to another address to change the port.
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
if (-not $env:SS2_PROFILE) { $env:SS2_PROFILE = '1' }
$viewer = $null
if (Get-Command puffin_viewer -ErrorAction SilentlyContinue) {
    $viewer = Start-Process puffin_viewer -ArgumentList '--url', '127.0.0.1:8585' -PassThru
} else {
    Write-Warning 'puffin_viewer is not installed (cargo install --locked puffin_viewer); serving anyway'
}
try {
    cargo run -p ss2-app --features profile -- @args
} finally {
    if ($viewer -and -not $viewer.HasExited) { $viewer.Kill() }
}
