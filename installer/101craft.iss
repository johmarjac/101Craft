; Inno Setup script for the 101Craft Windows installer.
; Values are passed in by the release workflow:
;   /DAppVersion=0.1.0 /DArch=x64compatible|arm64 /DSourceExe=... /DOutName=...

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef Arch
  #define Arch "x64compatible"
#endif
#ifndef SourceExe
  #define SourceExe "..\target\release\101Craft.exe"
#endif
#ifndef OutName
  #define OutName "101Craft-setup"
#endif

[Setup]
AppId={{6B1C2F3A-7D4E-4E1B-9A0C-101C4AF70101}
AppName=101Craft
AppVersion={#AppVersion}
AppVerName=101Craft {#AppVersion}
AppPublisher=101Craft
DefaultDirName={autopf}\101Craft
DefaultGroupName=101Craft
DisableProgramGroupPage=yes
OutputBaseFilename={#OutName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed={#Arch}
ArchitecturesInstallIn64BitMode={#Arch}
PrivilegesRequiredOverridesAllowed=dialog
UninstallDisplayIcon={app}\101Craft.exe

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\templates\*.tpl"; DestDir: "{app}\Templates"; Flags: ignoreversion

[Icons]
Name: "{group}\101Craft"; Filename: "{app}\101Craft.exe"
Name: "{group}\{cm:UninstallProgram,101Craft}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\101Craft"; Filename: "{app}\101Craft.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\101Craft.exe"; Description: "{cm:LaunchProgram,101Craft}"; Flags: nowait postinstall skipifsilent
