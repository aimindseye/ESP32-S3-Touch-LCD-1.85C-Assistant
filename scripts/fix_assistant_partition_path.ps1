$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"
$sdkconfigPath = Join-Path $firmwareDir "sdkconfig.defaults"
$partitionPath = (Resolve-Path (Join-Path $firmwareDir "partitions.csv")).Path.Replace('\', '/')

$sdk = Get-Content $sdkconfigPath -Raw
$updated = $sdk

$updated = $updated -replace '(?m)^CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=.*$', "CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=`"$partitionPath`""
$updated = $updated -replace '(?m)^CONFIG_PARTITION_TABLE_FILENAME=.*$', "CONFIG_PARTITION_TABLE_FILENAME=`"$partitionPath`""

if ($updated -notmatch '(?m)^CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=') {
    $updated = $updated.TrimEnd() + "`nCONFIG_PARTITION_TABLE_CUSTOM_FILENAME=`"$partitionPath`"`n"
}
if ($updated -notmatch '(?m)^CONFIG_PARTITION_TABLE_FILENAME=') {
    $updated = $updated.TrimEnd() + "`nCONFIG_PARTITION_TABLE_FILENAME=`"$partitionPath`"`n"
}

if ($updated -ne $sdk) {
    Set-Content -Encoding ASCII $sdkconfigPath $updated
    (Get-Item $sdkconfigPath).LastWriteTime = (Get-Date).AddSeconds(-30)
    Write-Host "Updated partition path: $partitionPath"
} else {
    Write-Host "Partition path already current: $partitionPath"
}

$cargoConfigPath = Join-Path $firmwareDir ".cargo\config.toml"
if (Test-Path $cargoConfigPath) {
    $cargoConfig = Get-Content $cargoConfigPath -Raw
    $cargoUpdated = $cargoConfig -replace 'ESP_IDF_TOOLS_INSTALL_DIR = "custom:C:/e"', 'ESP_IDF_TOOLS_INSTALL_DIR = "global"'
    if ($cargoUpdated -ne $cargoConfig) {
        Set-Content -Encoding ASCII $cargoConfigPath $cargoUpdated
        Write-Host "Updated ESP_IDF_TOOLS_INSTALL_DIR for global ESP-IDF tooling"
    }
}
