@echo off
echo 正在停止 HePrint 服务...
taskkill /F /IM heprint.exe 2>nul
if errorlevel 1 (
    echo HePrint 服务未运行
) else (
    echo ✅ HePrint 服务已停止
)
