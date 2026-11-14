$ErrorActionPreference = 'Stop'

function Normalize-RelayPath([string]$Value) {
    $resolved = [IO.Path]::GetFullPath($Value).Replace('/', '\')
    if ($resolved.StartsWith('\\?\UNC\')) { return '\\' + $resolved.Substring(8) }
    if ($resolved.StartsWith('\\?\')) { return $resolved.Substring(4) }
    return $resolved
}

$exists = Test-Path -LiteralPath $env:RELAY_LINK
if ($env:RELAY_REMOVE -eq '1' -and -not $exists) { return }

$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($env:RELAY_LINK)
$expected = Normalize-RelayPath $env:RELAY_LAUNCHER
if ($exists) {
    $actual = Normalize-RelayPath $shortcut.TargetPath
    if ($actual -ne $expected -or $shortcut.Arguments -ne $env:RELAY_ARGS) {
        throw 'This shortcut belongs to another installation.'
    }
}

if ($env:RELAY_REMOVE -eq '1') {
    Remove-Item -LiteralPath $env:RELAY_LINK
} else {
    $shortcut.TargetPath = $expected
    $shortcut.Arguments = $env:RELAY_ARGS
    $shortcut.Description = $env:RELAY_NAME
    if ($env:RELAY_ICON) { $shortcut.IconLocation = Normalize-RelayPath $env:RELAY_ICON }
    $shortcut.Save()
}
