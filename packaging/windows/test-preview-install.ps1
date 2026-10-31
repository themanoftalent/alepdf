param([Parameter(Mandatory)] [string] $Installer)
$ErrorActionPreference = 'Stop'
$Setup = (Resolve-Path -LiteralPath $Installer).Path
$Folder = Join-Path $env:RUNNER_TEMP ('AlePdfAcceptance-' + [Guid]::NewGuid().ToString('N'))
$Process = $null
try {
  $Install = Start-Process -FilePath $Setup -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',"/DIR=`"$Folder`"") -Wait -PassThru
  if ($Install.ExitCode -ne 0) { throw "Installer exit $($Install.ExitCode)" }
  & (Join-Path $Folder 'alepdf-cli.exe') --version
  if ($LASTEXITCODE -ne 0) { throw 'Installed CLI failed.' }
  $Process = Start-Process -FilePath (Join-Path $Folder 'alepdf.exe') -PassThru
  Start-Sleep -Seconds 8
  $Process.Refresh()
  if ($Process.HasExited) { throw "GUI exited during startup: $($Process.ExitCode)" }
  if ($Process.MainWindowHandle -eq 0) { throw 'No GUI window was detected on this Windows runner.' }
  Write-Output 'Per-user EXE installation, installed CLI and GUI window startup passed. Interactive PDF editing/signing not tested.'
} finally {
  if ($null -ne $Process -and -not $Process.HasExited) {
    [void] $Process.CloseMainWindow()
    if (-not $Process.WaitForExit(5000)) { Stop-Process -Id $Process.Id }
  }
  $Uninstall = Join-Path $Folder 'unins000.exe'
  if (Test-Path -LiteralPath $Uninstall) {
    $Remove = Start-Process -FilePath $Uninstall -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART') -Wait -PassThru
    if ($Remove.ExitCode -ne 0) { throw "Uninstaller exit $($Remove.ExitCode)" }
    if (Test-Path -LiteralPath (Join-Path $Folder 'alepdf.exe')) { throw 'Uninstall left app executable behind.' }
    Write-Output 'Preview uninstall passed.'
  }
}
