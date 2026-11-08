param(
    [Parameter(Mandatory = $true)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')][string]$Repository,
    [Parameter(Mandatory = $true)][ValidatePattern('^[a-fA-F0-9]{40,64}$')][string]$Commit,
    [string]$ArtifactDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist'),
    [switch]$Prerelease
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$tag = "v$Version"
$assets = @("WUp-$Version-windows-x64.exe", "WUp-$Version-windows-x64.msi", 'SHA256SUMS.txt', 'build-info.json')
$paths = @($assets | ForEach-Object {
    $path = Join-Path $ArtifactDirectory $_
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Release asset missing: $_" }
    (Resolve-Path -LiteralPath $path).Path
})
# Refuse to attach binaries to a pre-existing tag for a different commit.
$errorFile = Join-Path ([IO.Path]::GetTempPath()) ('wup-release-' + [guid]::NewGuid().ToString('N') + '.err')
$tagRef = & gh api "repos/$Repository/git/ref/tags/$tag" 2> $errorFile
$tagExit = $LASTEXITCODE
if ($tagExit -eq 0) {
    $tagCommit = & gh api "repos/$Repository/commits/$tag" --jq '.sha'
    if ($LASTEXITCODE -ne 0) { throw "Unable to resolve existing tag $tag." }
    if (($tagCommit | Out-String).Trim() -ne $Commit) { throw "Tag $tag points to a different commit." }
} elseif ((Get-Content -LiteralPath $errorFile -Raw) -notmatch 'HTTP 404') {
    throw "Unable to verify tag $tag. Check GitHub authentication and repository permissions."
}
$existingJson = & gh release view $tag --repo $Repository --json isDraft,url,assets,targetCommitish 2> $errorFile
$release = $null
if ($LASTEXITCODE -eq 0) { $release = $existingJson | ConvertFrom-Json }
if ($release -and $release.isDraft -and $release.targetCommitish -ne $Commit) {
    throw "Existing draft $tag targets a different commit."
}
if (-not $release) {
    $notesPath = Join-Path $ArtifactDirectory 'release-notes.md'
    @"
# WUp v$Version

Windows x64 WinGet Package Manager.

- EXE installer (MSI embedded): WUp-$Version-windows-x64.exe
- MSI installer: WUp-$Version-windows-x64.msi
- Commit: $Commit
- SHA256SUMS.txt contains package checksums.
"@ | Set-Content -LiteralPath $notesPath -Encoding utf8
    $createArguments = @('release', 'create', $tag, '--repo', $Repository, '--target', $Commit,
        '--title', "WUp v$Version", '--notes-file', $notesPath, '--draft')
    if ($Prerelease) { $createArguments += '--prerelease' }
    & gh @createArguments
    if ($LASTEXITCODE -ne 0) { throw 'Draft release creation failed.' }
}
if (-not $release -or $release.isDraft) {
    # Only drafts are replaced on retry. Publish after every asset has uploaded.
    & gh release upload $tag @paths --repo $Repository --clobber
    if ($LASTEXITCODE -ne 0) { throw 'Release asset upload failed; the release remains a draft.' }
    $editArguments = @('release', 'edit', $tag, '--repo', $Repository, '--draft=false')
    if ($Prerelease) { $editArguments += @('--prerelease', '--latest=false') }
    & gh @editArguments
    if ($LASTEXITCODE -ne 0) { throw 'Release publication failed.' }
} else {
    # Published releases keep their existing files, including immutable releases.
    $missing = @($paths | Where-Object { [IO.Path]::GetFileName($_) -notin $release.assets.name })
    if ($missing.Count -gt 0) {
        & gh release upload $tag @missing --repo $Repository
        if ($LASTEXITCODE -ne 0) { throw 'Could not upload missing assets to the existing release.' }
    }
}
$url = & gh release view $tag --repo $Repository --json url --jq '.url'
if ($LASTEXITCODE -ne 0) { throw 'Could not read the published release URL.' }
Write-Host "Published $tag`: $url"
if ($env:GITHUB_STEP_SUMMARY) { "Release: [$tag]($url)" >> $env:GITHUB_STEP_SUMMARY }
