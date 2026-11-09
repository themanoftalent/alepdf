<# Validate the actual portable archive on Windows; does not install or change associations. #>
param([Parameter(Mandatory)] [string] $Path)
$ErrorActionPreference = 'Stop'
$Archive = (Resolve-Path -LiteralPath $Path).Path
$Scratch = Join-Path ([IO.Path]::GetTempPath()) ('alepdf-portable-check-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $Scratch | Out-Null
try {
  Expand-Archive -LiteralPath $Archive -DestinationPath $Scratch
  $Folders = @(Get-ChildItem -LiteralPath $Scratch -Directory)
  if ($Folders.Count -ne 1) { throw 'Expected one portable package directory.' }
  $Root = $Folders[0].FullName
  foreach ($Name in 'alepdf.exe', 'alepdf-cli.exe', 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'LICENSE-app-icon.txt', 'LICENSE-document.txt', 'OFL-Inter.txt', 'OFL-JetBrainsMono.txt', 'OFL-DancingScript.txt', 'portable.txt', 'models/ATTRIBUTION.txt') {
    $File = Join-Path $Root $Name
    if (-not (Test-Path -LiteralPath $File -PathType Leaf)) { throw "Missing required portable payload: $Name" }
    if ((Get-Item -LiteralPath $File).Length -eq 0) { throw "Empty portable payload: $Name" }
  }
  foreach ($Name in 'alepdf.exe', 'alepdf-cli.exe') {
    $Bytes = [IO.File]::ReadAllBytes((Join-Path $Root $Name))
    if ($Bytes.Length -lt 64) { throw "Truncated PE: $Name" }
    $Offset = [BitConverter]::ToInt32($Bytes, 0x3c)
    if ($Offset -lt 0 -or $Offset -gt $Bytes.Length - 96) { throw "Invalid PE offset: $Name" }
    if ([BitConverter]::ToUInt32($Bytes, $Offset) -ne 0x00004550) { throw "Invalid PE signature: $Name" }
    if ([BitConverter]::ToUInt16($Bytes, $Offset + 4) -ne 0x8664) { throw "Not x64: $Name" }
  }
  & (Join-Path $Root 'alepdf-cli.exe') --version
  if ($LASTEXITCODE -ne 0) { throw 'Extracted CLI failed --version.' }
  Get-FileHash -LiteralPath $Archive -Algorithm SHA256 | Format-List
  Write-Output 'Portable extraction, required notices, x64 headers and extracted CLI startup passed. GUI/PDF runtime acceptance still required.'
} finally {
  Remove-Item -LiteralPath $Scratch -Recurse -Force
}
