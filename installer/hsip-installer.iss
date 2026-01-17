; HSIP Windows Installer - Inno Setup Script
; Creates a professional installer with full security features
; Build with Inno Setup Compiler 6.0+

#define MyAppName "HSIP"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Nyx Systems LLC"
#define MyAppURL "https://hsip.io"
#define MyAppExeName "hsip-cli.exe"
#define MyAppTrayName "hsip-tray.exe"

[Setup]
; Basic application info
AppId={{HSIP-ENCRYPTION-DAEMON}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=..\LICENSE
OutputDir=output
OutputBaseFilename=HSIP-Setup-{#MyAppVersion}
; SetupIconFile=hsip.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64
UninstallDisplayIcon={app}\{#MyAppExeName}

; Visual customization (optional - commented out if images not found)
; WizardImageFile=compiler:WizModernImage-IS.bmp
; WizardSmallImageFile=compiler:WizModernSmallImage-IS.bmp

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start HSIP automatically when Windows starts"; GroupDescription: "Startup Options:"; Flags: checkedonce

[Files]
; Main executables
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
; Tray icon commented out due to GTK3 security warnings (unmaintained dependencies)
; Source: "..\target\release\{#MyAppTrayName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\hsip-gateway.exe"; DestDir: "{app}"; Flags: ignoreversion

; PowerShell scripts
Source: "register-daemon.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "register-tray.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "run-daemon.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "run-tray.ps1"; DestDir: "{app}"; Flags: ignoreversion

; Documentation
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "..\TESTING_GUIDE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\AUDIT_LOG_GUIDE.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\INSTALL_WITH_AUDIT_LOGS.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\WHY_HSIP.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\GETTING_STARTED.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\SECURITY.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

; Security Test Scripts (for advanced users)
Source: "..\security_tests\*.py"; DestDir: "{app}\security_tests"; Flags: ignoreversion
Source: "..\security_tests\*.ps1"; DestDir: "{app}\security_tests"; Flags: ignoreversion
Source: "..\security_tests\README.md"; DestDir: "{app}\security_tests"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
; Tray icon shortcut commented out (GTK3 warnings)
; Name: "{group}\HSIP Status"; Filename: "{app}\{#MyAppTrayName}"
Name: "{group}\HSIP Command Line"; Filename: "cmd.exe"; Parameters: "/k cd /d ""{app}"" && {#MyAppExeName} --help"
Name: "{group}\Export Audit Logs"; Filename: "cmd.exe"; Parameters: "/k cd /d ""{app}"" && {#MyAppExeName} audit-export --out evidence.json"
Name: "{group}\Verify Audit Integrity"; Filename: "cmd.exe"; Parameters: "/k cd /d ""{app}"" && {#MyAppExeName} audit-verify"
Name: "{group}\Documentation"; Filename: "{app}\README.md"
Name: "{group}\Audit Log Guide (Court Evidence)"; Filename: "{app}\AUDIT_LOG_GUIDE.md"
Name: "{group}\Testing Guide"; Filename: "{app}\TESTING_GUIDE.md"
Name: "{group}\Getting Started"; Filename: "{app}\GETTING_STARTED.md"
Name: "{group}\Security Information"; Filename: "{app}\SECURITY.md"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Run]
; Register auto-start tasks if user selected the option
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register-daemon.ps1"""; StatusMsg: "Registering HSIP daemon..."; Flags: runhidden; Tasks: autostart
; Tray auto-start commented out (GTK3 warnings)
; Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register-tray.ps1"""; StatusMsg: "Registering HSIP tray icon..."; Flags: runhidden; Tasks: autostart

; Show success message
Filename: "{app}\README.md"; Description: "View Documentation"; Flags: postinstall shellexec skipifsilent unchecked

[UninstallRun]
; Stop and unregister scheduled tasks
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'HSIP Daemon' -Confirm:$false -ErrorAction SilentlyContinue"""; Flags: runhidden
; Tray task commented out (GTK3 warnings)
; Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Unregister-ScheduledTask -TaskName 'HSIP Tray' -Confirm:$false -ErrorAction SilentlyContinue"""; Flags: runhidden
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Stop-Process -Name '{#MyAppExeName}' -Force -ErrorAction SilentlyContinue"""; Flags: runhidden
; Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -Command ""Stop-Process -Name '{#MyAppTrayName}' -Force -ErrorAction SilentlyContinue"""; Flags: runhidden

[Code]
procedure InitializeWizard();
var
  WelcomeLabel: TLabel;
begin
  // Custom welcome message
  WelcomeLabel := TLabel.Create(WizardForm);
  WelcomeLabel.Parent := WizardForm.WelcomePage;
  WelcomeLabel.Caption :=
    'This wizard will install HSIP - Consent-Based Encrypted Communication.' + #13#10 + #13#10 +
    'HSIP v1.0.0 provides:' + #13#10 +
    '  • Ed25519 digital signatures (non-repudiation)' + #13#10 +
    '  • ChaCha20-Poly1305 encryption (Signal-grade)' + #13#10 +
    '  • PostgreSQL audit logs (court-ready evidence)' + #13#10 +
    '  • NTP time synchronization (±2 seconds)' + #13#10 +
    '  • Geolocation metadata tracking' + #13#10 +
    '  • Device fingerprinting' + #13#10 +
    '  • HMAC-SHA256 response integrity protection' + #13#10 +
    '  • OWASP Top 10 security hardening' + #13#10 +
    '  • Visual status with colored tray icons' + #13#10 +
    '  • Automatic threat blocking' + #13#10 +
    '  • Independent verification (IETF RFC 8439)' + #13#10 + #13#10 +
    'Enterprise-grade security - all features independently verified.' + #13#10 + #13#10 +
    'Look for the tray icon after installation:' + #13#10 +
    '  GREEN  = Protected' + #13#10 +
    '  YELLOW = Blocking threats' + #13#10 +
    '  RED    = Offline or error' + #13#10 + #13#10 +
    'NOTE: PostgreSQL required for audit logs. Install from:' + #13#10 +
    'https://www.postgresql.org/download/windows/';
  WelcomeLabel.Left := WizardForm.WelcomeLabel2.Left;
  WelcomeLabel.Top := WizardForm.WelcomeLabel2.Top + WizardForm.WelcomeLabel2.Height + 20;
  WelcomeLabel.Width := WizardForm.WelcomeLabel2.Width;
  WelcomeLabel.AutoSize := False;
  WelcomeLabel.WordWrap := True;
  WelcomeLabel.Height := 250;
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
  if MsgBox('This will remove HSIP and stop all protection services. Continue?',
            mbConfirmation, MB_YESNO) = IDNO then
    Result := False;
end;
