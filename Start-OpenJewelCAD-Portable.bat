@echo off
setlocal

set "APP_ROOT=%~dp0"
set "KERNEL_DIR=%APP_ROOT%kernel"
set "KERNEL_DLL=%KERNEL_DIR%\openjewelcad_kernel.dll"
set "APP_EXE=%APP_ROOT%openjewelcad.exe"

if not exist "%KERNEL_DLL%" (
    echo Open Jewel CAD's geometry engine is missing from the package.
    pause
    exit /b 1
)
if not exist "%APP_EXE%" (
    echo The Open Jewel CAD application is missing from the package.
    pause
    exit /b 1
)

set "OPENJEWELCAD_KERNEL_DLL=%KERNEL_DLL%"
set "PATH=%KERNEL_DIR%;%PATH%"
start "" "%APP_EXE%" gui
endlocal
