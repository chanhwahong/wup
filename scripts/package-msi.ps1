param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected a numeric MSI version.' }
$previousRollForward = $env:DOTNET_ROLL_FORWARD
if (-not $env:DOTNET_ROLL_FORWARD) { $env:DOTNET_ROLL_FORWARD = 'Major' }
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $exe = (Resolve-Path -LiteralPath 'target\release\wup.exe').Path
    $fileVersion = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
    if ($fileVersion -ne $Version) { throw "EXE version $fileVersion does not match requested MSI version $Version." }
    New-Item -ItemType Directory -Force -Path 'dist' | Out-Null
    $iconPath = (Resolve-Path -LiteralPath 'assets\wup.ico').Path
    dotnet tool run wix build installer\WUp.wxs -arch x64 -d "ProductVersion=$Version" -d "ExePath=$exe" -d "IconPath=$iconPath" -out "dist\WUp-$Version-windows-x64.msi"
    if ($LASTEXITCODE -ne 0) { throw 'MSI build failed. Run dotnet tool restore first.' }
    $msiPath = (Resolve-Path -LiteralPath "dist\WUp-$Version-windows-x64.msi").Path
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase($msiPath, 0)
    $view = $database.OpenView('SELECT `Value` FROM `Property` WHERE `Property` = ''ProductVersion''')
    $view.Execute()
    $record = $view.Fetch()
    if ($record.StringData(1) -ne $Version) { throw 'MSI product version validation failed.' }
    $view.Close()
    Write-Host "Verified MSI product version $Version"

} finally {
    $env:DOTNET_ROLL_FORWARD = $previousRollForward
    Pop-Location
}
