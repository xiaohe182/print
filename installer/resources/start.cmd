@echo off
setlocal
set "DIR=%~dp0"
echo.
echo ========================================
echo   HePrint v1.0.0 - 启动服务
echo ========================================
echo.

if not exist "%DIR%heprint.exe" (
    echo ❌ 找不到 heprint.exe
    pause
    exit /b 1
)

:: 检查是否已运行
tasklist /FI "IMAGENAME eq heprint.exe" 2>NUL | find /I "heprint.exe" >NUL
if not errorlevel 1 (
    echo ⚠ HePrint 已在运行
    echo 是否重启？[Y/N]
    set /p choice=
    if /i "!choice!"=="Y" (
        call "%DIR%stop.cmd"
    ) else (
        exit /b 0
    )
)

echo 🚀 启动 HePrint 服务（端口 18000）...
start "HePrint" /B "%DIR%heprint.exe" --port 18000

timeout /t 2 /nobreak >nul

echo.
echo ✅ HePrint 服务已在后台启动
echo.
echo 📋 下一步：
echo   1. 双击 "%DIR%index.html" 打开测试页
echo   2. 或访问 http://127.0.0.1:18000/ 查看服务状态
echo.
echo 💡 停止服务：双击 stop.cmd
echo.
pause
