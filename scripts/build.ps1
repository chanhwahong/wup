param([switch]$Installer)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    if (-not $env:GPUI_FXC_PATH) {
        $sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
        $compiler = Get-ChildItem -Path (Join-Path $sdkBin '*\x64\fxc.exe') -ErrorAction SilentlyContinue |
            Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
        if (-not $compiler) { throw 'Windows SDK fxc.exe was not found. Install the Windows 10/11 SDK.' }
        $env:GPUI_FXC_PATH = $compiler.FullName
    }
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'Rust formatting check failed.' }
    & (Join-Path $PSScriptRoot 'test-core.ps1')
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $version = ([regex]::Match([IO.File]::ReadAllText((Join-Path (Get-Location) 'Cargo.toml')), '(?m)^version = "([0-9.]+)"')).Groups[1].Value
    if (-not $version) { throw 'Package version not found.' }
    New-Item -ItemType Directory -Path 'dist' -Force | Out-Null
    $exe = "WUp-$version-windows-x64.exe"
    Copy-Item -LiteralPath 'target\release\wup.exe' -Destination (Join-Path 'dist' $exe) -Force
    Copy-Item -LiteralPath 'target\release\wup.exe' -Destination 'dist\wup.exe' -Force
    $packages = @((Join-Path 'dist' $exe))
    if ($Installer) {
        & (Join-Path $PSScriptRoot 'package-msi.ps1') -Version $version
        $packages += Join-Path 'dist' "WUp-$version-windows-x64.msi"
    }
    $packages | ForEach-Object {
        $hash = Get-FileHash -LiteralPath $_ -Algorithm SHA256
        "$($hash.Hash.ToLowerInvariant())  $([IO.Path]::GetFileName($_))"
    } | Set-Content -LiteralPath 'dist\SHA256SUMS.txt' -Encoding ascii
    [ordered]@{
        name = 'WUp'; version = $version; architecture = 'x64'
        commit = $env:GITHUB_SHA; run_number = $env:GITHUB_RUN_NUMBER
        run_attempt = $env:GITHUB_RUN_ATTEMPT
    } | ConvertTo-Json | Set-Content -LiteralPath 'dist\build-info.json' -Encoding utf8
    Write-Host "Built WUp $version in dist/"
} finally {
    Pop-Location
}
