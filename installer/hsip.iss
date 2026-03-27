; HSIP Windows Installer — built with Inno Setup 6
; No external file dependencies — works in any CI environment.

#define AppName    "HSIP"
#define AppVersion "1.0"
#define AppExe     "hsip.exe"

[Setup]
AppName                = {#AppName}
AppVersion             = {#AppVersion}
AppPublisher           = HSIP
AppPublisherURL        = https://github.com/rewired89/HSIP-1PHASE
AppSupportURL          = https://github.com/rewired89/HSIP-1PHASE/issues

; Install to the user's local app data — no admin password needed
DefaultDirName         = {localappdata}\HSIP
DefaultGroupName       = {#AppName}
PrivilegesRequired     = lowest

OutputDir              = output
OutputBaseFilename     = hsip-setup-windows

UninstallDisplayIcon   = {app}\{#AppExe}
UninstallDisplayName   = {#AppName}

; Appearance
WizardStyle            = modern
DisableDirPage         = yes
DisableProgramGroupPage= yes

Compression            = lzma2
SolidCompression       = yes

VersionInfoVersion     = 1.0.0.0
VersionInfoDescription = HSIP — Your Personal Data Security Hub

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "hsip-windows-x64.exe"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion

[Icons]
; Desktop shortcut
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Comment: "Open HSIP"
; Start Menu
Name: "{group}\{#AppName}";       Filename: "{app}\{#AppExe}"; Comment: "Open HSIP"
; Uninstall entry in Start Menu
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"

[Run]
; Offer to launch HSIP right after install
Filename: "{app}\{#AppExe}"; \
  Description: "Launch HSIP now"; \
  Flags: postinstall nowait skipifsilent

[UninstallRun]
; Stop HSIP before uninstalling
Filename: "taskkill"; Parameters: "/f /im {#AppExe}"; Flags: runhidden; RunOnceId: "KillHSIP"
