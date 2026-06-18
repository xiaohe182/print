@echo off
:: 在 Inno Setup 安装流程中，会自动以管理员运行
:: 此脚本作为 install.cmd 备份提供给 ZIP 用户

setlocal
set "DIR=%~dp0"

echo.
echo ========================================
echo   HePrint v1.0.0 - 安装到系统
echo ========================================
echo.

:: 检查管理员权限
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo ❌ 需要管理员权限！请右键以管理员身份运行。
    pause
    exit /b 1
)

:: 安装到 Program Files
set "TARGET=%ProgramFiles%\HePrint"
if not exist "%TARGET%" mkdir "%TARGET%"

echo 📦 复制文件到 %TARGET%...
xcopy /Y /E /I "%DIR%\*" "%TARGET%\" >nul
if errorlevel 1 (
    echo ❌ 复制失败
    pause
    exit /b 1
)

:: 注册防火墙
echo 🔧 注册 Windows 防火墙例外...
netsh advfirewall firewall delete rule name="HePrint Service" >nul 2>&1
netsh advfirewall firewall add rule name="HePrint Service" dir=in action=allow program="%TARGET%\heprint.exe" enable=yes >nul 2>&1

:: 启动服务
echo 🚀 启动 HePrint 服务...
start "HePrint" /B "%TARGET%\heprint.exe" --port 18000

timeout /t 2 /nobreak >nul

echo.
echo ✅ 安装完成！
echo.
echo 📋 快捷方式：
echo   启动服务: %TARGET%\start.cmd
echo   停止服务: %TARGET%\stop.cmd
echo   测试打印: %TARGET%\index.html
echo.
echo 🌐 浏览器访问: http://127.0.0.1:18000/
echo.
pause
