; HSIP Windows Installer - Inno Setup Script
; Company: Nyx Systems LLC
; Contact: nyxsystemsllc@gmail.com
; Build with Inno Setup Compiler 6.0+

#define MyAppName "HSIP"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Nyx Systems LLC"
#define MyAppURL "https://github.com/nyxsystems/HSIP-1PHASE"
#define MyAppContact "nyxsystemsllc@gmail.com"
#define MyAppExeName "hsip-cli.exe"

[Setup]
; Basic application info
AppId={{HSIP-SECURE-PROTOCOL-2026}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppContact={#MyAppContact}
; Per-user installation (no admin required)
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
AllowNoIcons=yes
LicenseFile=..\LICENSE
InfoBeforeFile=..\README.md
OutputDir=output
OutputBaseFilename=HSIP-Setup-{#MyAppVersion}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
; No icon files - using defaults
UninstallDisplayIcon={app}\{#MyAppExeName}
; Per-user registry location
ChangesEnvironment=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start HSIP automatically when Windows starts (silent background)"; GroupDescription: "Startup Options:"; Flags: checkedonce

[Files]
; Main executables
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\hsip-gateway.exe"; DestDir: "{app}"; Flags: ignoreversion

; Silent launcher
Source: "launch-hidden.vbs"; DestDir: "{app}"; Flags: ignoreversion

; Documentation
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\COMMERCIAL_LICENSE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\SECURITY_AUDIT.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\GETTING_STARTED.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\SECURITY.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\TESTING_GUIDE.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\AUDIT_LOG_GUIDE.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\security_tests\README.md"; DestDir: "{app}\security_tests"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
; Per-user Start Menu shortcuts (no admin required)
Name: "{userprograms}\{#MyAppName}\HSIP Command Line"; Filename: "cmd.exe"; Parameters: "/k cd /d ""{app}"" && {#MyAppExeName} --help"
Name: "{userprograms}\{#MyAppName}\Documentation"; Filename: "{app}\README.md"
Name: "{userprograms}\{#MyAppName}\License (Free Non-Commercial)"; Filename: "{app}\LICENSE"
Name: "{userprograms}\{#MyAppName}\Commercial License Info"; Filename: "{app}\COMMERCIAL_LICENSE.md"
Name: "{userprograms}\{#MyAppName}\Security Audit Report"; Filename: "{app}\SECURITY_AUDIT.md"
Name: "{userprograms}\{#MyAppName}\Getting Started"; Filename: "{app}\GETTING_STARTED.md"
Name: "{userprograms}\{#MyAppName}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Registry]
; Silent background startup using VBScript launcher (no console windows)
; Launches hsip-cli daemon in hidden mode
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
      ValueType: string; ValueName: "HSIP Daemon"; \
      ValueData: "wscript.exe ""{app}\launch-hidden.vbs"" ""{app}\hsip-cli.exe"" ""daemon"""; \
      Flags: uninsdeletevalue; Tasks: autostart

; Launches hsip-gateway in hidden mode
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; \
      ValueType: string; ValueName: "HSIP Gateway"; \
      ValueData: "wscript.exe ""{app}\launch-hidden.vbs"" ""{app}\hsip-gateway.exe"" """""; \
      Flags: uninsdeletevalue; Tasks: autostart

[Run]
; Show documentation after install
Filename: "{app}\README.md"; Description: "View Documentation"; Flags: postinstall shellexec skipifsilent unchecked

[UninstallRun]
; Stop processes before uninstall (RunOnceId prevents double execution)
Filename: "taskkill"; Parameters: "/F /IM hsip-cli.exe /T"; Flags: runhidden; RunOnceId: "stop_hsip_cli"; StatusMsg: "Stopping HSIP daemon..."
Filename: "taskkill"; Parameters: "/F /IM hsip-gateway.exe /T"; Flags: runhidden; RunOnceId: "stop_hsip_gateway"; StatusMsg: "Stopping HSIP gateway..."

[Code]
procedure InitializeWizard();
var
  WelcomeLabel: TLabel;
  InfoMemo: TNewMemo;
begin
  // Custom welcome page with security features and licensing info
  WelcomeLabel := TLabel.Create(WizardForm);
  WelcomeLabel.Parent := WizardForm.WelcomePage;
  WelcomeLabel.Caption :=
    'HSIP (Hyper Secure Internet Protocol)' + #13#10 +
    'Consent-Based Encrypted Communication' + #13#10 + #13#10 +
    'Version: 1.0.0' + #13#10 +
    'Publisher: Nyx Systems LLC' + #13#10 +
    'Contact: nyxsystemsllc@gmail.com' + #13#10 + #13#10 +
    'Security Features:' + #13#10 +
    '  • Ed25519 signatures / ChaCha20-Poly1305 encryption' + #13#10 +
    '  • Replay attack protection (nonce-based)' + #13#10 +
    '  • DoS/Injection defenses' + #13#10 +
    '  • OWASP Top 10 hardening' + #13#10 +
    '  • PostgreSQL audit logs (court-ready)' + #13#10 +
    '  • NTP time sync (±2 seconds)' + #13#10 + #13#10 +
    'Recent Updates:' + #13#10 +
    '  • Fixed panic vulnerability (DoS prevention)' + #13#10 +
    '  • Updated dependencies (RUSTSEC-2025-0132)' + #13#10 +
    '  • Enhanced error handling' + #13#10 + #13#10 +
    'LICENSE:' + #13#10 +
    '  FREE for personal, educational, non-commercial use' + #13#10 +
    '  Commercial use requires license from Nyx Systems LLC' + #13#10 +
    '  See LICENSE and COMMERCIAL_LICENSE.md' + #13#10 + #13#10 +
    'Silent Startup:' + #13#10 +
    '  HSIP will run in background - no console windows' + #13#10 +
    '  Daemon and gateway start automatically on login';

  WelcomeLabel.Left := WizardForm.WelcomeLabel2.Left;
  WelcomeLabel.Top := WizardForm.WelcomeLabel2.Top + WizardForm.WelcomeLabel2.Height + 20;
  WelcomeLabel.Width := WizardForm.WelcomeLabel2.Width;
  WelcomeLabel.AutoSize := False;
  WelcomeLabel.WordWrap := True;
  WelcomeLabel.Height := 320;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    // Create .hsip directory for config and logs
    CreateDir(ExpandConstant('{userappdata}\.hsip'));
  end;
end;

function InitializeUninstall(): Boolean;
begin
  Result := True;
  if MsgBox('This will remove HSIP and stop all protection services.' + #13#10 +
            'Your configuration and logs will be preserved in %APPDATA%\.hsip' + #13#10#13#10 +
            'Continue with uninstall?',
            mbConfirmation, MB_YESNO) = IDNO then
    Result := False;
end;
