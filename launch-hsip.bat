@echo off
:: ============================================================
:: HSIP Launcher
:: Double-click this file to start HSIP and open the app.
:: Place this .bat file in the same folder as hsip.exe
:: ============================================================

set "DIR=%~dp0"
set "EXE=%DIR%hsip-windows-x64.exe"
set "PORT=7474"
set "URL=http://localhost:%PORT%"

:: Check if hsip is already running on port 7474
netstat -an 2>nul | find ":%PORT%" | find "LISTENING" >nul 2>&1
if %errorlevel% == 0 (
    echo HSIP is already running. Opening browser...
    start "" "%URL%"
    exit /b 0
)

:: Make sure hsip.exe exists
if not exist "%EXE%" (
    echo ERROR: hsip-windows-x64.exe not found.
    echo Make sure launch-hsip.bat and hsip-windows-x64.exe are in the same folder.
    pause
    exit /b 1
)

:: Start HSIP silently in the background (no window)
echo Starting HSIP...
start "" /B "%EXE%"

:: Wait for it to be ready (up to 10 seconds)
set /a tries=0
:wait
timeout /t 1 /nobreak >nul
netstat -an 2>nul | find ":%PORT%" | find "LISTENING" >nul 2>&1
if %errorlevel% == 0 goto ready
set /a tries+=1
if %tries% lss 10 goto wait

echo HSIP took too long to start. Check %APPDATA%\HSIP\hsip.log for errors.
pause
exit /b 1

:ready
echo HSIP is ready. Opening browser...
start "" "%URL%"
