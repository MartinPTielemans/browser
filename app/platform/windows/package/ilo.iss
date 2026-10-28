; ilo's installer for winget and direct downloads, unsigned for 0.1.0: per user and without admin,
; into %LOCALAPPDATA%\Programs\ilo, with its uninstaller in the user's Apps list. WinSparkle runs a
; newer one over it. package.sh compiles it with ISCC, giving each value below.
#ifndef AppVersion
  #error ISCC /DAppName=ilo /DAppPublisher=ilo /DAppVersion=0.1.0 /DSource=<staged app> /DOutput=<dir> ilo.iss
#endif
; -app-only for Browser.dll packed without an engine, which doesn't run.
#ifndef Suffix
  #define Suffix ""
#endif

[Setup]
; Never changes: Windows and winget know the installed app by it (ProductCode app.ilo.browser_is1).
AppId=app.ilo.browser
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
VersionInfoVersion={#AppVersion}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
DefaultDirName={autopf}\{#AppName}
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
DisableWelcomePage=yes
; ilo running is closed for the update, as files in use can't be replaced.
CloseApplications=force
SetupIconFile=Assets\ilo.ico
UninstallDisplayIcon={app}\ilo.ico
UninstallDisplayName={#AppName}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
OutputDir={#Output}
OutputBaseFilename={#AppName}-{#AppVersion}-x64-setup{#Suffix}

[Files]
Source: "{#Source}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\chrome.exe"; IconFilename: "{app}\ilo.ico"

[Registry]
; So ilo can be the default browser. Per user (PrivilegesRequired=lowest), named IloHTML to match
; the ProgId host.cc's host_is_default_browser reads from the user's http/https UserChoice. Windows
; itself lets only the user set that choice, in Settings, which host_make_default_browser opens.
Root: HKCU; Subkey: "Software\Classes\IloHTML"; ValueType: string; ValueData: "ilo HTML Document"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\IloHTML\DefaultIcon"; ValueType: string; ValueData: "{app}\ilo.ico,0"
Root: HKCU; Subkey: "Software\Classes\IloHTML\shell\open\command"; ValueType: string; ValueData: """{app}\chrome.exe"" ""%1"""
; The capabilities Windows lists in Settings' Default apps, so http, https and HTML files can be ilo's.
Root: HKCU; Subkey: "Software\ilo"; Flags: uninsdeletekeyifempty
Root: HKCU; Subkey: "Software\ilo\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "{#AppName}"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\ilo\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "{#AppName}, an Arc-style browser"
Root: HKCU; Subkey: "Software\ilo\Capabilities\URLAssociations"; ValueType: string; ValueName: "http"; ValueData: "IloHTML"
Root: HKCU; Subkey: "Software\ilo\Capabilities\URLAssociations"; ValueType: string; ValueName: "https"; ValueData: "IloHTML"
Root: HKCU; Subkey: "Software\ilo\Capabilities\FileAssociations"; ValueType: string; ValueName: ".htm"; ValueData: "IloHTML"
Root: HKCU; Subkey: "Software\ilo\Capabilities\FileAssociations"; ValueType: string; ValueName: ".html"; ValueData: "IloHTML"
; Register those capabilities with Windows, so it shows ilo as a choice.
Root: HKCU; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "{#AppName}"; ValueData: "Software\ilo\Capabilities"; Flags: uninsdeletevalue

[Run]
Filename: "{app}\chrome.exe"; Description: "Open {#AppName}"; Flags: nowait postinstall skipifsilent
