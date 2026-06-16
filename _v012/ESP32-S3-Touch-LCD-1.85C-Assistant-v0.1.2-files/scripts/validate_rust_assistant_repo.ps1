$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$firmwareDir = Join-Path $repoRoot "firmware\assistant-rs"

function Require-File([string]$path) {
    if (-not (Test-Path $path -PathType Leaf)) {
        throw "Missing required file: $path"
    }
}

function Require-Dir([string]$path) {
    if (-not (Test-Path $path -PathType Container)) {
        throw "Missing required directory: $path"
    }
}

Require-Dir $firmwareDir

$requiredFiles = @(
    "Cargo.toml",
    "build.rs",
    ".cargo\config.toml",
    "sdkconfig.defaults",
    "partitions.csv",
    "src\main.rs",
    "src\app\mod.rs",
    "src\app\model.rs",
    "src\app\pages.rs",
    "components\st77916_shim\CMakeLists.txt",
    "components\st77916_shim\st77916_shim.c",
    "components\st77916_shim\esp_lcd_st77916.c"
)

foreach ($relative in $requiredFiles) {
    Require-File (Join-Path $firmwareDir $relative)
}

$cargo = Get-Content (Join-Path $firmwareDir "Cargo.toml") -Raw
if ($cargo -notmatch 'version\s*=\s*"0\.1\.2"') {
    throw "Cargo.toml version must be 0.1.2"
}
if ($cargo -notmatch 'extra_components') {
    throw "Cargo.toml missing esp-idf-sys extra_components metadata"
}
if ($cargo -notmatch 'components/st77916_shim') {
    throw "Cargo.toml missing components/st77916_shim extra component"
}

$sdk = Get-Content (Join-Path $firmwareDir "sdkconfig.defaults") -Raw
foreach ($pattern in @(
    'CONFIG_SPIRAM=y',
    'CONFIG_ESPTOOLPY_FLASHSIZE_16MB=y',
    'CONFIG_PARTITION_TABLE_CUSTOM=y',
    'CONFIG_PARTITION_TABLE_CUSTOM_FILENAME=',
    'CONFIG_PARTITION_TABLE_FILENAME=',
    'CONFIG_PARTITION_TABLE_OFFSET=0x8000'
)) {
    if ($sdk -notmatch [regex]::Escape($pattern)) {
        throw "sdkconfig.defaults missing $pattern"
    }
}

$partitionFilename = [regex]::Match($sdk, '(?m)^CONFIG_PARTITION_TABLE_FILENAME="?([^"\r\n]+)"?').Groups[1].Value
if ([string]::IsNullOrWhiteSpace($partitionFilename)) {
    throw "Unable to parse CONFIG_PARTITION_TABLE_FILENAME"
}
if (-not (Test-Path $partitionFilename -PathType Leaf)) {
    throw "Partition table path does not exist: $partitionFilename. Run scripts\fix_assistant_partition_path.ps1"
}

$partitions = Get-Content (Join-Path $firmwareDir "partitions.csv") -Raw
if ($partitions -notmatch 'factory,\s*app,\s*factory,\s*0x10000,\s*0x300000') {
    throw "partitions.csv must contain a 3MB factory app partition"
}
if ($partitions -notmatch 'storage,\s*data,\s*fat,\s*0x310000,\s*0xCF0000') {
    throw "partitions.csv must preserve the remaining flash as FAT storage"
}

$main = Get-Content (Join-Path $firmwareDir "src\main.rs") -Raw
foreach ($pattern in @(
    'AssistantPage',
    'draw_assistant_page',
    'draw_placeholder_page',
    'page_from_point',
    'Home/Weather/Music/Settings'
)) {
    if ($main -notmatch [regex]::Escape($pattern)) {
        throw "src/main.rs missing app shell marker: $pattern"
    }
}

$pages = Get-Content (Join-Path $firmwareDir "src\app\pages.rs") -Raw
foreach ($page in @('Home', 'Weather', 'Music', 'Settings')) {
    if ($pages -notmatch $page) {
        throw "src/app/pages.rs missing page: $page"
    }
}

Write-Host "Rust assistant repo validation: OK"
