$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"
$sdkconfigPath = Join-Path $firmwareDir "sdkconfig.defaults"
$partitionPath = (Resolve-Path (Join-Path $firmwareDir "partitions.csv")).Path.Replace('\', '/')

$sdk = Get-Content $sdkconfigPath -Raw
$sdk = $sdk -replace '(?m)^CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=.*$', "CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=`"$partitionPath`""
$sdk = $sdk -replace '(?m)^CONFIG_PARTITION_TABLE_FILENAME=.*$', "CONFIG_PARTITION_TABLE_FILENAME=`"$partitionPath`""

if ($sdk -notmatch '(?m)^CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=') {
    $sdk = $sdk.TrimEnd() + "`nCONFIG_PARTITION_TABLE_CUSTOM_FILENAME=`"$partitionPath`"`n"
}
if ($sdk -notmatch '(?m)^CONFIG_PARTITION_TABLE_FILENAME=') {
    $sdk = $sdk.TrimEnd() + "`nCONFIG_PARTITION_TABLE_FILENAME=`"$partitionPath`"`n"
}

Set-Content -Encoding ASCII $sdkconfigPath $sdk
Write-Host "Updated partition path: $partitionPath"
