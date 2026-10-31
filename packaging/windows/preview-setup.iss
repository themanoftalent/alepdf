; Owner-review Windows x64 EXE installer. Compile only after validating the portable payload.
#ifndef PayloadDir
  #error PayloadDir must name the validated portable directory
#endif
#ifndef ReleaseDir
  #error ReleaseDir is required
#endif
#ifndef AppVersion
  #error AppVersion is required
#endif
[Setup]
AppId=AlePdf.Windows.Preview
AppName=AlePdf Preview
AppVersion={#AppVersion}
AppPublisher=AlePdf contributors
AppPublisherURL=https://github.com/themanoftalent/alepdf
AppSupportURL=https://github.com/themanoftalent/alepdf/issues
DefaultDirName={localappdata}\Programs\AlePdfPreview
DefaultGroupName=AlePdf Preview
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#ReleaseDir}
OutputBaseFilename=AlePdf-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\alepdf.exe
CloseApplications=no
RestartApplications=no
AllowNoIcons=yes
DisableProgramGroupPage=yes
[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{group}\AlePdf Preview"; Filename: "{app}\alepdf.exe"
; No file associations, services, scheduled tasks, automatic launch, or security overrides.
