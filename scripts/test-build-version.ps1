$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$temp = Join-Path ([IO.Path]::GetTempPath()) ('wup-version-tests-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
$manifest = [IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'))
$lock = [IO.File]::ReadAllText((Join-Path $root 'Cargo.lock'))
$base = [version]([regex]::Match($manifest, '(?m)^version = "([0-9.]+)"').Groups[1].Value)
$utf8 = [Text.UTF8Encoding]::new($false)
function Reset-Fixture {
    [IO.File]::WriteAllText((Join-Path $temp 'Cargo.toml'), $manifest, $utf8)
    [IO.File]::WriteAllText((Join-Path $temp 'Cargo.lock'), $lock, $utf8)
}
foreach ($number in @(1, 2, 10)) {
    Reset-Fixture
    $actual = & (Join-Path $PSScriptRoot 'set-build-version.ps1') -BuildNumber $number -ProjectRoot $temp
    $expected = "$($base.Major).$($base.Minor).$($base.Build + $number)"
    if ($actual -ne $expected) { throw "Expected $expected, got $actual" }
    $updatedLock = [IO.File]::ReadAllText((Join-Path $temp 'Cargo.lock'))
    $rootPattern = '(?m)(^name = "wup"\r?\nversion = ")[^"]+("\r?$)'
    $restoredLock = [regex]::Replace($updatedLock, $rootPattern, { param($m) $m.Groups[1].Value + $base.ToString() + $m.Groups[2].Value })
    if ($restoredLock -ne $lock) { throw 'Unrelated lockfile entries changed.' }
    $updatedManifest = [IO.File]::ReadAllText((Join-Path $temp 'Cargo.toml'))
    if ($updatedManifest -notmatch [regex]::Escape("version = `"$expected`"")) { throw 'Manifest was not updated.' }
}
foreach ($number in @(0, 65536)) {
    Reset-Fixture
    $rejected = $false
    try { & (Join-Path $PSScriptRoot 'set-build-version.ps1') -BuildNumber $number -ProjectRoot $temp | Out-Null }
    catch { $rejected = $true }
    if (-not $rejected) { throw "Invalid number $number was accepted." }
    if ([IO.File]::ReadAllText((Join-Path $temp 'Cargo.toml')) -ne $manifest) { throw 'Rejected build changed the manifest.' }
}
Write-Host 'Build version tests passed (increments, lockfile consistency and MSI bounds).'
