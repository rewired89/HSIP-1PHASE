; HSIP Windows Installer — built with Inno Setup 6
; Installs without requiring administrator rights.
; Creates Desktop + Start Menu shortcuts automatically.

#define AppName    "HSIP"
#define AppVersion "1.0"
#define AppExe     "hsip.exe"

[Setup]
AppName                = {#AppName}
AppVersion             = {#AppVersion}
AppPublisher           = HSIP
AppPublisherURL        = https://github.com/rewired89/HSIP-1PHASE
AppSupportURL          = https://github.com/rewired89/HSIP-1PHASE/issues
AppUpdatesURL          = https://github.com/rewired89/HSIP-1PHASE/releases

; Install to the user's local app data — no admin password needed
DefaultDirName         = {localappdata}\HSIP
DefaultGroupName       = {#AppName}
PrivilegesRequired     = lowest
PrivilegesRequiredOverridesAllowed = dialog

OutputDir              = .
OutputBaseFilename     = hsip-setup-windows
SetupIconFile          = hsip.ico
UninstallDisplayIcon   = {app}\{#AppExe}
UninstallDisplayName   = {#AppName}

; Appearance
WizardStyle            = modern
WizardSmallImageFile   = hsip-installer-banner.bmp
DisableWelcomePage     = no
DisableDirPage         = yes
DisableProgramGroupPage= yes

Compression            = lzma2/ultra64
SolidCompression       = yes

; Version info shown in Add/Remove Programs
VersionInfoVersion     = 1.0.0.0
VersionInfoDescription = HSIP — Your Personal Data Security Hub
VersionInfoCopyright   = HSIP

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
english.WelcomeLabel2=HSIP protects your digital privacy.%n%nThis will install HSIP on your computer. Once installed, you can open it anytime from the Desktop shortcut or your Start Menu.%n%nClick Install to continue.

[Files]
Source: "..\hsip-windows-x64.exe"; DestDir: "{app}"; DestName: "{#AppExe}"; Flags: ignoreversion

[Icons]
; Desktop shortcut (created for all users of this machine)
Name: "{autodesktop}\{#AppName}";    Filename: "{app}\{#AppExe}"; Comment: "Open HSIP — your personal data security hub"
; Start Menu shortcut
Name: "{group}\{#AppName}";          Filename: "{app}\{#AppExe}"; Comment: "Open HSIP — your personal data security hub"
; Start Menu — Uninstall
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"

[Run]
; Offer to launch HSIP immediately after install
Filename: "{app}\{#AppExe}"; \
  Description: "Launch HSIP now"; \
  Flags: postinstall nowait skipifsilent

[UninstallRun]
; Stop HSIP if it's running when the user uninstalls
Filename: "taskkill"; Parameters: "/f /im {#AppExe}"; Flags: runhidden; RunOnceId: "KillHSIP"
