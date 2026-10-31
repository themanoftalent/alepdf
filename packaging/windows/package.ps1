<#
.SYNOPSIS
  Build, sign and package PdfCraft for Windows.

.DESCRIPTION
  Produces, in $env:DIST (default: dist/release):
    pdfcraft-<version>-windows-<arch>.msi            per-machine installer (WiX v5)
    pdfcraft-<version>-windows-<arch>-portable.zip   pdfcraft.exe + pdfcraft-cli.exe + models\ + portable.txt

  The binaries link the C runtime statically (+crt-static), so neither the MSI nor the portable
  zip needs the Visual C++ redistributable. Signing is delegated to sign.ps1 (skipped with a
  warning when no signing secrets are set).

  Needs: Rust (MSVC toolchain + the target), curl and the network (the OCR models), the Windows SDK (rc.exe, signtool.exe),
  and WiX v5: dotnet tool install --global wix --version 5.0.2

.EXAMPLE
  pwsh packaging/windows/package.ps1 -Arch x64
  pwsh packaging/windows/package.ps1 -Arch x86 -SkipBuild   # set CRAFT_FONTS_DIR, or -BuiltWithoutCraftFonts
  pwsh packaging/windows/package.ps1 -Arch arm64     # cross-compiled; needs the MSVC ARM64 build tools
#>
param(
  [ValidateSet('x64', 'x86', 'arm64')] [string] $Arch = 'x64',
  [switch] $SkipBuild,
  # Portable preview builds do not install WiX or mutate installer registration.
  [switch] $PortableOnly,
  # -SkipBuild only: the existing binaries were built without CRAFT_FONTS_DIR, so they embed no
  # craft-fonts faces. Without CRAFT_FONTS_DIR this assertion is required: package.ps1 cannot
  # otherwise know whether the binaries need the fonts' OFL notices.
  [switch] $BuiltWithoutCraftFonts
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

if ($BuiltWithoutCraftFonts -and -not $SkipBuild) {
  throw '-BuiltWithoutCraftFonts describes an existing build made without CRAFT_FONTS_DIR; use it only with -SkipBuild.'
}
if ($BuiltWithoutCraftFonts -and $env:CRAFT_FONTS_DIR) {
  throw '-BuiltWithoutCraftFonts cannot be combined with CRAFT_FONTS_DIR.'
}
if ($SkipBuild -and -not $env:CRAFT_FONTS_DIR -and -not $BuiltWithoutCraftFonts) {
  throw 'package.ps1 -SkipBuild needs CRAFT_FONTS_DIR to stage the OFL notices of embedded craft-fonts, or -BuiltWithoutCraftFonts when the binaries were built without it.'
}

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

# The version lives in one place: [workspace.package] version in the root Cargo.toml.
$Version = $env:PDFCRAFT_VERSION
if (-not $Version) {
  $inPkg = $false
  foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
    if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[workspace.package]'); continue }
    if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
  }
}
if (-not $Version) { throw 'could not read [workspace.package] version from Cargo.toml' }
# MSI ProductVersion is numeric (major.minor.build); pre-release tags are dropped there.
$MsiVersion = ($Version -split '-')[0]

$Target = switch ($Arch) { 'x64' { 'x86_64-pc-windows-msvc' } 'x86' { 'i686-pc-windows-msvc' } 'arm64' { 'aarch64-pc-windows-msvc' } }
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
New-Item -ItemType Directory -Force -Path $Dist | Out-Null

if (-not $env:PDFCRAFT_BUILD_SHA) { $env:PDFCRAFT_BUILD_SHA = (git -C $Root rev-parse HEAD 2>$null) }
if (-not $env:PDFCRAFT_BUILD_DATE) { $env:PDFCRAFT_BUILD_DATE = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd') }

Write-Output "PdfCraft $Version for Windows $Arch ($Target)"

if (-not $SkipBuild) {
  # Static CRT: no VC++ redistributable needed. Scoped to the target so host build scripts and
  # proc-macros are unaffected.
  $flagVar = 'CARGO_TARGET_' + ($Target.ToUpper() -replace '-', '_') + '_RUSTFLAGS'
  [Environment]::SetEnvironmentVariable($flagVar, '-C target-feature=+crt-static')
  # Fail the build (rather than warn) if the icon/VERSIONINFO can't be embedded.
  $env:PDFCRAFT_REQUIRE_WINRES = '1'
  Invoke-Native "cargo build ($Target)" { cargo build --release --locked -p pdfcraft -p pdfcraft-cli --target $Target }
}

$Bin = Join-Path $TargetDir "$Target\release"

# Check both binaries' PE headers before packaging. Machine (COFF header) must match -Arch, so an
# x64 build can never ship labelled arm64 (#61). Subsystem (optional header): 2 = Windows GUI,
# 3 = console. The app must be GUI (no console window opens with it, #57); the CLI must stay
# console so its output reaches the terminal.
function Get-PeHeader([string] $Path) {
  $bytes = [System.IO.File]::ReadAllBytes($Path)
  $pe = [BitConverter]::ToInt32($bytes, 0x3C)
  return @{ Machine = [BitConverter]::ToUInt16($bytes, $pe + 4); Subsystem = [BitConverter]::ToUInt16($bytes, $pe + 0x5C) }
}
$Machine = switch ($Arch) { 'x64' { 0x8664 } 'x86' { 0x14C } 'arm64' { 0xAA64 } }
if (Test-Path (Join-Path $Bin 'alepdf.exe')) {
  Copy-Item (Join-Path $Bin 'alepdf.exe') (Join-Path $Bin 'pdfcraft.exe') -Force
}
if (Test-Path (Join-Path $Bin 'alepdf-cli.exe')) {
  Copy-Item (Join-Path $Bin 'alepdf-cli.exe') (Join-Path $Bin 'pdfcraft-cli.exe') -Force
} elseif (Test-Path (Join-Path $Bin 'pdfcraft-cli.exe')) {
  Copy-Item (Join-Path $Bin 'pdfcraft-cli.exe') (Join-Path $Bin 'alepdf-cli.exe') -Force
}

foreach ($check in @(@('pdfcraft.exe', 2), @('pdfcraft-cli.exe', 3))) {
  $h = Get-PeHeader (Join-Path $Bin $check[0])
  if ($h.Machine -ne $Machine) { throw "$($check[0]) is for machine 0x$('{0:X}' -f $h.Machine), expected 0x$('{0:X}' -f $Machine) ($Arch)" }
  if ($h.Subsystem -ne $check[1]) { throw "$($check[0]) has PE subsystem $($h.Subsystem), expected $($check[1])" }
  Write-Output "ok $($check[0]): $Arch, PE subsystem $($h.Subsystem)"
}
$Stage = Join-Path $TargetDir "windows-package\$Arch"
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item (Join-Path $Bin 'pdfcraft.exe'), (Join-Path $Bin 'pdfcraft-cli.exe') $Stage
if (Test-Path (Join-Path $Bin 'alepdf.exe')) { Copy-Item (Join-Path $Bin 'alepdf.exe') $Stage }
if (Test-Path (Join-Path $Bin 'alepdf-cli.exe')) { Copy-Item (Join-Path $Bin 'alepdf-cli.exe') $Stage }
# The document icon embeds Lucide artwork: carry its ISC notice and the project licences
# beside the binaries in both distributions.
foreach ($f in 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE') {
  Copy-Item (Join-Path $Root $f) $Stage
}
Copy-Item (Join-Path $Root 'assets\app-icon\LICENSE-document.txt') $Stage

& (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'pdfcraft.exe') (Join-Path $Stage 'pdfcraft-cli.exe')
if (Test-Path (Join-Path $Stage 'alepdf.exe')) { & (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'alepdf.exe') }
if (Test-Path (Join-Path $Stage 'alepdf-cli.exe')) { & (Join-Path $PSScriptRoot 'sign.ps1') (Join-Path $Stage 'alepdf-cli.exe') }

# OCR models (#103): the app looks in models\ beside pdfcraft.exe. `cargo xtask models` fetches every
# ATTRIBUTION.toml `kind = "model"` file (verified by SHA-256) with its licence and ATTRIBUTION.txt;
# the MSI and the portable zip take the whole folder, so new models ship without naming them here.
$Models = Join-Path $Stage 'models'
New-Item -ItemType Directory -Force -Path $Models | Out-Null
Push-Location $Root
try { Invoke-Native 'cargo xtask models' { cargo xtask models $Models } } finally { Pop-Location }
Remove-Item -Force (Join-Path $Models '*.part') -ErrorAction SilentlyContinue
if (-not (Test-Path (Join-Path $Models 'ATTRIBUTION.txt')) -or -not (Get-ChildItem (Join-Path $Models '*.LICENCE.txt'))) {
  throw "no OCR models in $Models after cargo xtask models"
}

# ---- font notices for the MSI ------------------------------------------------------------------
# The portable ZIP already carries these notices; stage the complete files before MSI creation too.
$Docs = Join-Path $Stage 'font-licences'
New-Item -ItemType Directory -Force -Path $Docs | Out-Null
$HasCraftFontLicences = 0
if ($env:CRAFT_FONTS_DIR) {
  . (Join-Path $PSScriptRoot 'font-notices.ps1')
  # Validate both native manifests before copying anything. One family's notice cannot
  # stand in for another's; retain complete files, including copyright and reserved names.
  $fontNotices = @(Get-CraftFontNotices $env:CRAFT_FONTS_DIR (Test-Path Env:CRAFT_FONTS_REQUIRED))
  foreach ($ofl in $fontNotices) {
    Copy-Item -LiteralPath $ofl.FullName -Destination (Join-Path $Docs "OFL-$($ofl.Directory.Name).txt")
    $HasCraftFontLicences = 1
  }
}
# ---- end font notices for the MSI --------------------------------------------------------------

# ---- MSI ---------------------------------------------------------------------------------------
if (-not $PortableOnly) {
$Msi = Join-Path $Dist "AlePdf-$Version-windows-$Arch.msi"
Invoke-Native 'wix build' {
  wix build (Join-Path $PSScriptRoot 'pdfcraft.wxs') -arch $Arch `
    (Join-Path $PSScriptRoot 'installer-ui.wxs') `
    -d "Version=$MsiVersion" -d "BinDir=$Stage" -d "ModelsDir=$Models" -d "IconPath=$(Join-Path $Root 'assets\app-icon\pdfcraft.ico')" `
    -d "DocsDir=$Docs" -d "HasCraftFontLicences=$HasCraftFontLicences" `
    -o $Msi
}
# Also copy to legacy name for compatibility if needed
Copy-Item $Msi (Join-Path $Dist "pdfcraft-$Version-windows-$Arch.msi") -Force

if ($env:RUN_UPSTREAM_TESTS) {
  Invoke-Native 'test-msi' {
    & (Get-Process -Id $PID).Path -NoProfile -File (Join-Path $PSScriptRoot 'test-msi.ps1') $Msi `
      -CraftFontLicences $HasCraftFontLicences -ExpectedFontNoticesDirectory $Docs
  }
  & (Join-Path $PSScriptRoot 'check-install-policy.ps1') -MsiPath $Msi
}
# wix writes its debug symbols (.wixpdb) next to the MSI; keep them out of the release assets.
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))
& (Join-Path $PSScriptRoot 'sign.ps1') $Msi

}

# ---- portable zip ------------------------------------------------------------------------------
$Portable = Join-Path $TargetDir "windows-package\AlePdf-$Version-windows-$Arch-portable"
Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Portable | Out-Null
Copy-Item (Join-Path $Stage '*.exe') $Portable
foreach ($f in 'NOTICE', 'LICENSE-document.txt') {
  Copy-Item (Join-Path $Stage $f) $Portable
}
Copy-Item -Recurse $Models (Join-Path $Portable 'models')
foreach ($f in 'README.md', 'LICENSE', 'LICENSE-MIT', 'LICENSE-APACHE') {
  $p = Join-Path $Root $f
  if (Test-Path $p) { Copy-Item $p $Portable }
}
if ($HasCraftFontLicences -eq 1) {
  Copy-Item (Join-Path $Docs 'OFL-*.txt') $Portable
}
# Carry the licences for embedded original app artwork and all bundled Latin fonts.
Copy-Item (Join-Path $Root 'assets\app-icon\LICENSE.txt') (Join-Path $Portable 'LICENSE-app-icon.txt')
Copy-Item (Join-Path $Root 'assets\fonts\OFL-*.txt') $Portable
Copy-Item (Join-Path $PSScriptRoot 'portable.txt') $Portable
# Preserve dependency notices from the exact resolved package sources, including egui fonts.
Push-Location $Root
try {
  $MetadataJson = cargo metadata --locked --format-version 1 --filter-platform $Target
  if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve dependency licence sources.' }
  $Metadata = ($MetadataJson -join "`n") | ConvertFrom-Json
} finally { Pop-Location }
$LegalRoot = Join-Path $Portable 'THIRD-PARTY-LICENSES'
New-Item -ItemType Directory -Force -Path $LegalRoot | Out-Null
foreach ($Package in $Metadata.packages) {
  $PackageRoot = Split-Path $Package.manifest_path
  $PackageLegal = Join-Path $LegalRoot "$($Package.name)-$($Package.version)"
  $LegalFiles = @(Get-ChildItem -LiteralPath $PackageRoot -File | Where-Object { $_.Name -match '(?i)licen[sc]e|^copying|^notice|^copyright' })
  $Fonts = Join-Path $PackageRoot 'fonts'
  if (Test-Path -LiteralPath $Fonts -PathType Container) {
    $LegalFiles += @(Get-ChildItem -LiteralPath $Fonts -File -Filter '*.txt')
  }
  if ($Package.license_file) {
    $Declared = Join-Path $PackageRoot $Package.license_file
    if (-not (Test-Path -LiteralPath $Declared)) { throw "Missing declared licence file for $($Package.name)" }
    $LegalFiles += Get-Item -LiteralPath $Declared
  }
  foreach ($LegalFile in ($LegalFiles | Sort-Object FullName -Unique)) {
    $Relative = [IO.Path]::GetRelativePath($PackageRoot, $LegalFile.FullName)
    if ($Relative.StartsWith('..')) { throw "Licence escapes package root: $($Package.name)" }
    $Destination = Join-Path $PackageLegal $Relative
    New-Item -ItemType Directory -Force -Path (Split-Path $Destination) | Out-Null
    Copy-Item -LiteralPath $LegalFile.FullName -Destination $Destination
  }
}
foreach ($Required in 'UFL.txt', 'Hack-Regular.txt', 'OFL.txt', 'emoji-icon-font-mit-license.txt') {
  if (-not (Get-ChildItem -LiteralPath $LegalRoot -Recurse -File -Filter $Required)) { throw "Missing bundled font notice: $Required" }
}
Copy-Item (Join-Path $Root 'assets\icons\LICENSE-lucide.txt') $Portable
Copy-Item (Join-Path $Root 'vendor\hayro-cmap\assets\LICENSE.txt') (Join-Path $Portable 'LICENSE-CMap.txt')
Copy-Item (Join-Path $Root 'vendor\hayro-interpret\assets\LICENSE_FOXIT') $Portable
Copy-Item (Join-Path $Root 'vendor\hayro-interpret\assets\CGATS_LICENSE.txt') $Portable
$Zip = Join-Path $Dist "AlePdf-$Version-windows-$Arch-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Portable -DestinationPath $Zip
Copy-Item $Zip (Join-Path $Dist "pdfcraft-$Version-windows-$Arch-portable.zip") -Force

# Smoke-test the CLI when this machine can run it. An ARM64 build made on an x64 runner can't run
# here; .github/workflows/windows-arm64.yml installs and runs it on ARM64 instead.
$HostArch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString().ToLowerInvariant()
if ($Arch -ne 'arm64' -or $HostArch -eq 'arm64') {
  Invoke-Native 'pdfcraft-cli --version' { & (Join-Path $Stage 'pdfcraft-cli.exe') --version }
} else {
  Write-Output "skipping pdfcraft-cli --version: an $Arch build doesn't run on this $HostArch machine"
}
if ($PortableOnly) { Get-Item $Zip | Format-Table Name, Length }
else { Get-Item $Msi, $Zip | Format-Table Name, Length }
