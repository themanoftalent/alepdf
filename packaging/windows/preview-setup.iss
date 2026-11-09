; AlePdf Windows x64 Inno Setup Installer
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
AppId=AlePdf.Windows
AppName=AlePdf
AppVersion={#AppVersion}
AppPublisher=Mehmet Akif Cifci
AppPublisherURL=https://github.com/themanoftalent/alepdf
AppSupportURL=https://github.com/themanoftalent/alepdf/issues
DefaultDirName={localappdata}\Programs\AlePdf
DefaultGroupName=AlePdf
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#ReleaseDir}
OutputBaseFilename=AlePdf-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\alepdf.exe
CloseApplications=no
RestartApplications=no
AllowNoIcons=yes
DisableProgramGroupPage=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\AlePdf"; Filename: "{app}\alepdf.exe"
Name: "{autodesktop}\AlePdf"; Filename: "{app}\alepdf.exe"; Tasks: desktopicon

; Clean, secure installer:
; - No administrative elevation required (installs in user %LOCALAPPDATA%\Programs\AlePdf)
; - No adware, telemetry, or hidden network services
; - Standard, reversible uninstaller with full cleanup
; - Direct Start Menu shortcut

