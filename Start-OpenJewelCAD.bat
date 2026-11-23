@echo off
setlocal

set "APP_ROOT=%~dp0"
pushd "%APP_ROOT%" >nul
if errorlevel 1 (
    echo Could not open the Open Jewel CAD folder.
    pause
    exit /b 1
)

set "BUILD_CONFIG=Debug"
set "CARGO_PROFILE=debug"
if not exist "%APP_ROOT%build\Debug\openjewelcad_kernel.dll" (
    if exist "%APP_ROOT%build\Release\openjewelcad_kernel.dll" (
        set "BUILD_CONFIG=Release"
        set "CARGO_PROFILE=release"
    )
)

set "KERNEL_DIR=%APP_ROOT%build\%BUILD_CONFIG%"
set "KERNEL_DLL=%KERNEL_DIR%\openjewelcad_kernel.dll"
if not exist "%KERNEL_DLL%" (
    echo Open Jewel CAD's OCCT kernel has not been built yet.
    echo Build the native kernel using the Windows instructions in README.md,
    echo then double-click this file again.
    pause
    popd
    exit /b 1
)

set "APP_EXE=%APP_ROOT%target\%CARGO_PROFILE%\openjewelcad.exe"
where cargo >nul 2>nul
if errorlevel 1 (
    if not exist "%APP_EXE%" (
        echo Rust Cargo was not found and the desktop app has not been built.
        echo Install Rust or build the app using the Windows instructions in README.md.
        pause
        popd
        exit /b 1
    )
) else (
    echo Building the Open Jewel CAD desktop app...
    if /I "%BUILD_CONFIG%"=="Release" (
        cargo build --locked --release -p openjewelcad-cli
    ) else (
        cargo build --locked -p openjewelcad-cli
    )
    if errorlevel 1 (
        echo The app build failed. Check the messages above and the README instructions.
        pause
        popd
        exit /b 1
    )
)

set "OPENJEWELCAD_KERNEL_DLL=%KERNEL_DLL%"
set "PATH=%KERNEL_DIR%;%PATH%"
start "" "%APP_EXE%" gui
if errorlevel 1 (
    echo Could not start Open Jewel CAD.
    pause
    popd
    exit /b 1
)

popd
endlocal
