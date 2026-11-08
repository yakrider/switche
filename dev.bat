@echo off
REM fast dev loop: stop any running debug instance, rebuild, and relaunch it shown
REM (this never touches the installer/release packaging)
setlocal
cd /d "%~dp0"

powershell -NoProfile -Command "Get-CimInstance Win32_Process | Where-Object { $_.Name -eq 'Switche.exe' -and $_.ExecutablePath -like '*\target\debug\Switche.exe' } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force }; Start-Sleep -Milliseconds 800" >nul 2>&1

cargo build --manifest-path switche-back\Cargo.toml
if errorlevel 1 (
  timeout /t 1 /nobreak >nul
  cargo build --manifest-path switche-back\Cargo.toml
  if errorlevel 1 exit /b 1
)

start "" "switche-back\target\debug\Switche.exe"
