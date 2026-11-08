param(
    [Parameter(Mandatory = $true)][long]$BuildNumber,
    [string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot)
)
$ErrorActionPreference = 'Stop'
if ($BuildNumber -lt 1) { throw 'BuildNumber must be positive.' }
$manifestPath = Join-Path $ProjectRoot 'Cargo.toml'
$lockPath = Join-Path $ProjectRoot 'Cargo.lock'
$manifest = [IO.File]::ReadAllText($manifestPath)
$versionPattern = '(?m)^version = "(\d+)\.(\d+)\.(\d+)"\r?$'
$match = [regex]::Match($manifest, $versionPattern)
if ([regex]::Matches($manifest, $versionPattern).Count -ne 1) { throw 'Expected one stable package version in Cargo.toml.' }
$major = [long]$match.Groups[1].Value
$minor = [long]$match.Groups[2].Value
$patch = [long]$match.Groups[3].Value
if ($major -gt 255 -or $minor -gt 255 -or $patch -gt 65535 -or $BuildNumber -gt (65535 - $patch)) {
    throw 'Version exceeds MSI limits (255.255.65535). Bump the minor version and reset the base patch.'
}
$version = "$major.$minor.$($patch + $BuildNumber)"
$manifest = [regex]::new($versionPattern).Replace($manifest, "version = `"$version`"", 1)
$lock = [IO.File]::ReadAllText($lockPath)
$packagePattern = '(?m)(^name = "wup"\r?\nversion = ")[^"]+("\r?$)'
if ([regex]::Matches($lock, $packagePattern).Count -ne 1) { throw 'Expected exactly one wup entry in Cargo.lock.' }
$lock = [regex]::Replace($lock, $packagePattern, { param($m) $m.Groups[1].Value + $version + $m.Groups[2].Value })
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText($manifestPath, $manifest, $utf8)
[IO.File]::WriteAllText($lockPath, $lock, $utf8)
if ($env:GITHUB_OUTPUT) { "version=$version" >> $env:GITHUB_OUTPUT }
Write-Output $version
