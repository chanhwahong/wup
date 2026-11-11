param()
$ErrorActionPreference = 'Stop'
$testName = 'wup-core-' + [guid]::NewGuid().ToString('N')
$harness = Join-Path ([System.IO.Path]::GetTempPath()) ($testName + '.rs')
$testBinary = Join-Path ([System.IO.Path]::GetTempPath()) ($testName + '.exe')
$projectRoot = Split-Path -Parent $PSScriptRoot
$modules = foreach ($moduleName in @('i18n', 'winget', 'scrollbar', 'theme', 'ui_scale', 'pixel_snap', 'package_filter', 'batch_progress')) {
    $modulePath = (Join-Path $projectRoot ('src/' + $moduleName + '.rs')).Replace([char]92, [char]47)
    '#[path = "' + $modulePath + '"] mod ' + $moduleName + ';'
}
Set-Content -LiteralPath $harness -Value (@('#![allow(dead_code)]') + $modules) -Encoding utf8
rustc --edition 2024 --test --crate-name wup_core_tests $harness -o $testBinary
if ($LASTEXITCODE -ne 0) { throw 'Core test compilation failed.' }
& $testBinary
if ($LASTEXITCODE -ne 0) { throw 'Core tests failed.' }
