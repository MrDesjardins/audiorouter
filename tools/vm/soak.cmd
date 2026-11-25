@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0soak.ps1" %*
