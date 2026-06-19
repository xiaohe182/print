@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
cd /d "%SCRIPT_DIR%"

set "DIST=%SCRIPT_DIR%dist"
set "STAGE=%DIST%\heprint-v1.1.1"
set "RES=%SCRIPT_DIR%installer\resources"

echo.
echo ========================================
echo   HePrint v1.1.1 Build Script
echo ========================================
echo.

if not exist "target\release\heprint.exe" (
    echo [ERROR] heprint.exe not found at:
    echo         %SCRIPT_DIR%target\release\heprint.exe
    echo Please run: cargo build --release
    pause
    exit /b 1
)

if exist "%DIST%" rmdir /S /Q "%DIST%"
mkdir "%DIST%"
mkdir "%STAGE%"
mkdir "%STAGE%\web-sdk"
mkdir "%STAGE%\cert"

echo [1/4] Copying files...
copy /Y "target\release\heprint.exe" "%STAGE%\heprint.exe" >nul
copy /Y "%RES%\install.cmd" "%STAGE%\install.cmd" >nul
copy /Y "%RES%\uninstall.cmd" "%STAGE%\uninstall.cmd" >nul
copy /Y "%RES%\start.cmd" "%STAGE%\start.cmd" >nul
copy /Y "%RES%\stop.cmd" "%STAGE%\stop.cmd" >nul
copy /Y "web-sdk\heprint.js" "%STAGE%\web-sdk\heprint.js" >nul
copy /Y "index.html" "%STAGE%\index.html" >nul
copy /Y "README.md" "%STAGE%\README.md" >nul
copy /Y "design-doc.md" "%STAGE%\design-doc.md" >nul
copy /Y "quick-start.md" "%STAGE%\quick-start.md" >nul
copy /Y "%RES%\cert\README.txt" "%STAGE%\cert\README.txt" >nul
echo       OK

echo.
echo [2/4] Validating heprint.exe...
for %%I in ("%STAGE%\heprint.exe") do echo       heprint.exe = %%~zI bytes

echo.
echo [3/4] Building ZIP package...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Compress-Archive -Path '%STAGE%\*' -DestinationPath '%DIST%\heprint-v1.1.1.zip' -Force"
if exist "%DIST%\heprint-v1.1.1.zip" (
    for %%I in ("%DIST%\heprint-v1.1.1.zip") do echo       ZIP: %%~zI bytes
) else (
    echo       [WARN] ZIP failed
)

echo.
echo [4/4] Listing portable bundle...
echo       Bundle: %STAGE%

echo.
echo ========================================
echo   Build Complete
echo ========================================
echo.
echo Output:
echo   ZIP:     %DIST%\heprint-v1.1.1.zip
echo   Bundle:  %STAGE%\
echo.
echo Install:
echo   1. Unzip to any location
echo   2. Run install.cmd as Administrator
echo   3. Run start.cmd to launch service
echo   4. Open index.html to test printing
echo.
echo ========================================
pause
