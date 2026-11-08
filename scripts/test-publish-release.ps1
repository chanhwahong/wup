$ErrorActionPreference = 'Stop'
$previousSummary = $env:GITHUB_STEP_SUMMARY
$env:GITHUB_STEP_SUMMARY = $null
$folder = Join-Path ([IO.Path]::GetTempPath()) ('wup-publish-tests-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $folder | Out-Null
$assetNames = @('WUp-0.1.2-windows-x64.exe', 'WUp-0.1.2-windows-x64.msi', 'SHA256SUMS.txt', 'build-info.json')
$assetNames | ForEach-Object { [IO.File]::WriteAllText((Join-Path $folder $_), 'test fixture') }
$global:wupReleaseTestCommit = 'a' * 40
$global:wupReleaseTestAssets = @($assetNames | ForEach-Object { @{ name = $_ } })
function global:gh {
    $global:wupReleaseTestCalls.Add(@($args))
    $global:LASTEXITCODE = 0
    if ($args[0] -eq 'api') {
        if ($global:wupReleaseTestCase -eq 'new') {
            Write-Error 'gh: Not Found (HTTP 404)' -ErrorAction Continue
            $global:LASTEXITCODE = 1
        } elseif ($global:wupReleaseTestCase -eq 'wrong-tag') { Write-Output ('b' * 40) }
        else { Write-Output $global:wupReleaseTestCommit }
    } elseif ($args[1] -eq 'view') {
        if ($args -contains '--jq') { Write-Output 'https://github.com/test/wup/releases/tag/v0.1.2' }
        elseif ($global:wupReleaseTestCase -eq 'new') { $global:LASTEXITCODE = 1 }
        else {
            @{ isDraft = ($global:wupReleaseTestCase -ne 'published'); assets = $global:wupReleaseTestAssets
               targetCommitish = $global:wupReleaseTestCommit; url = 'https://example.test/release' } | ConvertTo-Json -Depth 4
        }
    } elseif ($args[1] -eq 'upload' -and $global:wupReleaseTestCase -eq 'upload-failure') {
        $global:LASTEXITCODE = 1
    }
}
foreach ($case in @('new', 'draft', 'published', 'wrong-tag', 'upload-failure')) {
    $global:wupReleaseTestCase = $case
    $global:wupReleaseTestCalls = [Collections.Generic.List[object]]::new()
    $failed = $false
    try {
        & (Join-Path $PSScriptRoot 'publish-release.ps1') -Version '0.1.2' -Repository 'test/wup' `
            -Commit $global:wupReleaseTestCommit -ArtifactDirectory $folder -Prerelease
    } catch { $failed = $true }
    $calls = @($global:wupReleaseTestCalls | ForEach-Object { $_ -join ' ' })
    if ($case -in @('wrong-tag', 'upload-failure')) {
        if (-not $failed) { throw "Expected rejection: $case" }
        if ($calls -match '^release edit ') { throw 'Failed upload or wrong tag was published.' }
    } else {
        if ($failed) { throw "Unexpected failure: $case" }
        if ($case -eq 'published') {
            if ($calls -match '^release (upload|edit|create) ') { throw 'Published assets were changed on retry.' }
        } else {
            $uploadIndex = [array]::FindIndex([string[]]$calls, [Predicate[string]]{ param($x) $x -match '^release upload ' })
            $publishIndex = [array]::FindIndex([string[]]$calls, [Predicate[string]]{ param($x) $x -match '^release edit ' })
            if ($uploadIndex -lt 0 -or $publishIndex -le $uploadIndex) { throw 'Release must upload before publishing.' }
            if (-not ($calls -match '--latest=false')) { throw 'Prerelease must not replace Latest.' }
            if ($case -eq 'new' -and -not ($calls -match '--target a{40}')) { throw 'Tag was not pinned to the built commit.' }
        }
    }
}
Remove-Item Function:\gh
$env:GITHUB_STEP_SUMMARY = $previousSummary
Write-Host 'Publish tests passed: new release, draft retry, published retry, wrong tag and upload failure.'
