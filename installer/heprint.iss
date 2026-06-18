; ===============================================
; HePrint v1.0.0 Inno Setup 脚本
; 高级安装包：单文件 .exe 安装器
; 编译：需先安装 Inno Setup 6+ → https://jrsoftware.org/isinfo.php
; 然后用 Inno Setup 打开本文件 → 编译
; ===============================================

#define MyAppName "HePrint"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "HePrint Team"
#define MyAppURL "https://heprint.example.com"
#define MyAppExeName "heprint.exe"

[Setup]
; NOTE: AppId 是唯一标识，不要修改！
AppId={{8F5C8E1A-1B2C-4D3E-9F4A-7C5D6E7F8A9B}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}

; 安装路径
DefaultDirName={autopf}\HePrint
DefaultGroupName=HePrint

; 输出
OutputDir=dist
OutputBaseFilename=HePrint-v1.0.0-setup
SetupIconFile=resources\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}

; 压缩
Compression=lzma2/ultra
SolidCompression=yes

; 权限：管理员
PrivilegesRequired=admin
PrivilegesRequiredOverridesAllowed=dialog

; 架构
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible

; 美化
WizardStyle=modern
WizardSizePercent=120

[Languages]
Name: "chinesesimp"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; 主程序
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

; 启动/卸载脚本
Source: "installer\resources\install.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "installer\resources\uninstall.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "installer\resources\start.cmd"; DestDir: "{app}"; Flags: ignoreversion
Source: "installer\resources\stop.cmd"; DestDir: "{app}"; Flags: ignoreversion

; 前端 SDK
Source: "web-sdk\heprint.js"; DestDir: "{app}\web-sdk"; Flags: ignoreversion
Source: "index.html"; DestDir: "{app}"; Flags: ignoreversion

; 文档
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "设计文档.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "快速启动.md"; DestDir: "{app}"; Flags: ignoreversion

; 证书目录
Source: "installer\resources\cert\*"; DestDir: "{app}\cert"; Flags: ignoreversion

[Dirs]
Name: "{app}\logs"

[Icons]
; 开始菜单
Name: "{group}\HePrint 启动"; Filename: "{app}\start.cmd"
Name: "{group}\HePrint 停止"; Filename: "{app}\stop.cmd"
Name: "{group}\HePrint 测试页"; Filename: "{app}\index.html"
Name: "{group}\HePrint 卸载"; Filename: "{uninstallexe}"

; 桌面图标（可选）
Name: "{commondesktop}\HePrint"; Filename: "{app}\index.html"; Tasks: desktopicon

[Run]
; 安装后可选：启动服务
Filename: "{app}\start.cmd"; Description: "{cm:LaunchProgram,HePrint}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
; 卸载前停止服务
Filename: "{app}\stop.cmd"; Flags: runhidden

[UninstallDelete]
; 清理证书
Type: filesandordirs; Name: "{app}\cert"
Type: filesandordirs; Name: "{app}\logs"
; 询问用户是否保留配置
Type: filesandordirs; Name: "{app}\heprint.toml"

[Code]
// 卸载前询问
function InitializeUninstall(): Boolean;
begin
  Result := MsgBox(
    '确认卸载 HePrint？' + #13#10 + #13#10 +
    '将删除：C:\Program Files\HePrint\*' + #13#10 +
    '将停止：HePrint 服务进程' + #13#10 +
    '将清理：Windows 防火墙规则',
    mbConfirmation, MB_YESNO
  ) = IDYES;
end;

// 防火墙规则清理
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then begin
    Exec('netsh', 'advfirewall firewall delete rule name="HePrint Service"', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;

// 安装后注册防火墙例外
procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then begin
    Exec('netsh', 'advfirewall firewall add rule name="HePrint Service" dir=in action=allow program="C:\Program Files\HePrint\heprint.exe" enable=yes', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
