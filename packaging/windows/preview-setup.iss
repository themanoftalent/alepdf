; PdfAle Windows x64 Inno Setup Installer
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
AppId=PdfAle.Windows
AppName=PdfAle
AppVersion={#AppVersion}
AppPublisher=Mehmet Akif Cifci
AppPublisherURL=https://github.com/themanoftalent/pdfale
AppSupportURL=https://github.com/themanoftalent/pdfale/issues
DefaultDirName={localappdata}\Programs\PdfAle
DefaultGroupName=PdfAle
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible and not arm64
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#ReleaseDir}
OutputBaseFilename=PdfAle-{#AppVersion}-windows-x64-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\pdfale.exe
CloseApplications=no
RestartApplications=no
AllowNoIcons=yes
DisableProgramGroupPage=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#PayloadDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\PdfAle"; Filename: "{app}\pdfale.exe"
Name: "{autodesktop}\PdfAle"; Filename: "{app}\pdfale.exe"; Tasks: desktopicon

; Clean, secure installer:
; - No administrative elevation required (installs in user %LOCALAPPDATA%\Programs\PdfAle)
; - No adware, telemetry, or hidden network services
; - Standard, reversible uninstaller with full cleanup
; - Direct Start Menu shortcut

