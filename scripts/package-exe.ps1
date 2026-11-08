param([Parameter(Mandatory = $true)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version)
$ErrorActionPreference = 'Stop'
$previousRollForward = $env:DOTNET_ROLL_FORWARD
if (-not $env:DOTNET_ROLL_FORWARD) { $env:DOTNET_ROLL_FORWARD = 'Major' }
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    $msiPath = (Resolve-Path -LiteralPath "dist\WUp-$Version-windows-x64.msi").Path
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $database = $installer.OpenDatabase($msiPath, 0)
    $view = $database.OpenView('SELECT `Value` FROM `Property` WHERE `Property` = ''ProductVersion''')
    $view.Execute()
    $record = $view.Fetch()
    if ($record.StringData(1) -ne $Version) { throw 'MSI version does not match the EXE installer version.' }
    $view.Close()
    $iconPath = (Resolve-Path -LiteralPath 'assets\wup.ico').Path
    $logoPath = (Resolve-Path -LiteralPath 'assets\balloon.png').Path
    dotnet tool run wix extension add WixToolset.Bal.wixext/4.0.6
    if ($LASTEXITCODE -ne 0) { throw 'WiX bootstrapper extension restore failed.' }
    dotnet tool run wix build installer\Bundle.wxs -arch x64 -ext WixToolset.Bal.wixext `
        -d "ProductVersion=$Version" -d "MsiPath=$msiPath" -d "IconPath=$iconPath" -d "LogoPath=$logoPath" `
        -out "dist\WUp-$Version-windows-x64.exe"
    if ($LASTEXITCODE -ne 0) { throw 'EXE installer build failed.' }
    # Verify this is a real Burn installer and its embedded MSI is the exact package we built.
    $verifyDirectory = Join-Path ([IO.Path]::GetTempPath()) ('wup-exe-verify-' + [guid]::NewGuid().ToString('N'))
    dotnet tool run wix burn extract "dist\WUp-$Version-windows-x64.exe" -o $verifyDirectory -oba (Join-Path $verifyDirectory 'bootstrapper')
    if ($LASTEXITCODE -ne 0) { throw 'EXE installer extraction validation failed.' }
    $embedded = @(Get-ChildItem -LiteralPath $verifyDirectory -Recurse -Filter '*.msi' -File)
    if ($embedded.Count -ne 1) { throw 'EXE installer must contain exactly one MSI package.' }
    if ((Get-FileHash -LiteralPath $embedded[0].FullName -Algorithm SHA256).Hash -ne
        (Get-FileHash -LiteralPath $msiPath -Algorithm SHA256).Hash) {
        throw 'EXE installer payload differs from the standalone MSI.'
    }
    Write-Host "Built EXE installer with embedded MSI: WUp-$Version-windows-x64.exe"
} finally {
    $env:DOTNET_ROLL_FORWARD = $previousRollForward
    Pop-Location
}
