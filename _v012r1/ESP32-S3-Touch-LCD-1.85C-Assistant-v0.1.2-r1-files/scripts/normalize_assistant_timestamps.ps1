param(
    [int]$SecondsBack = 60
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"
$cutoff = (Get-Date).AddSeconds(-1 * [Math]::Abs($SecondsBack))

$paths = @(
    $firmwareDir,
    (Join-Path $repoRoot "scripts"),
    (Join-Path $repoRoot "docs")
)

foreach ($root in $paths) {
    if (-not (Test-Path $root)) {
        continue
    }

    Get-ChildItem -LiteralPath $root -Recurse -Force | Where-Object {
        $_.FullName -notmatch '\\target(\\|$)' -and
        $_.FullName -notmatch '\\\.git(\\|$)' -and
        $_.FullName -notmatch '\\_v012(\\|$)'
    } | ForEach-Object {
        try {
            $_.LastWriteTime = $cutoff
        } catch {
            Write-Warning "Could not normalize timestamp: $($_.FullName)"
        }
    }

    try {
        (Get-Item -LiteralPath $root).LastWriteTime = $cutoff
    } catch {
        Write-Warning "Could not normalize timestamp: $root"
    }
}

Write-Host "Normalized assistant source timestamps to $cutoff"
