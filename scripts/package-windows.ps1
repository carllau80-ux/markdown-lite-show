param(
    [string]$Version = "0.1.4",
    [string]$Binary = "target\release\mdlite.exe"
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$BinaryPath = Join-Path $Root $Binary
if (-not (Test-Path $BinaryPath)) {
    throw "Missing executable: $BinaryPath"
}

$Stage = Join-Path $Root "dist\windows\MarkLite"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item $BinaryPath (Join-Path $Stage "mdlite.exe")
Copy-Item (Join-Path $Root "README.md") $Stage
Copy-Item (Join-Path $Root "LICENSE") $Stage

$Archive = Join-Path $Root "dist\MarkLite-$Version-windows-x64.zip"
if (Test-Path $Archive) { Remove-Item -Force $Archive }
Compress-Archive -Path $Stage -DestinationPath $Archive
Write-Output $Archive
