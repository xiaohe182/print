@echo off
setlocal

echo.
echo ========================================
echo   HePrint v1.1.1 - 卸载
echo ========================================
echo.

:: 停止服务
echo 🛑 停止 HePrint 服务...
taskkill /F /IM heprint.exe 2>nul

:: 清理防火墙规则
echo 🔧 清理防火墙规则...
netsh advfirewall firewall delete rule name="HePrint Service" >nul 2>&1

:: 删除安装目录
set "TARGET=%ProgramFiles%\HePrint"
if exist "%TARGET%" (
    echo 🗑️  删除 %TARGET%...
    rmdir /S /Q "%TARGET%" 2>nul
)

echo.
echo ✅ 卸载完成
echo.
pause
