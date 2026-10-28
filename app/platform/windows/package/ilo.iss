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

[Run]
Filename: "{app}\chrome.exe"; Description: "Open {#AppName}"; Flags: nowait postinstall skipifsilent
