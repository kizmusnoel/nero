@echo off
setlocal enabledelayedexpansion

:: ============================================================
:: Build Rust UEFI app and deploy it to a USB drive
:: Usage: build_and_deploy.bat <DRIVE_LETTER>
:: Example: build_and_deploy.bat E
:: ============================================================

if "%~1"=="" (
    echo Usage: %~nx0 ^<DRIVE_LETTER^>
    echo Example: %~nx0 E
    exit /b 1
)

set "DRIVE=%~1:"
set "PROJECT_DIR=."
set "BIN_NAME=nero"
set "TARGET=x86_64-unknown-uefi"
set "BUILD_OUT=%PROJECT_DIR%\target\%TARGET%\release\%BIN_NAME%.efi"

:: --- sanity check the drive exists ---
if not exist "%DRIVE%\" (
    echo ERROR: Drive %DRIVE% not found. Check the letter and try again.
    exit /b 1
)

:: --- safety check: refuse to touch C: no matter what ---
if /i "%DRIVE%"=="C:" (
    echo ERROR: Refusing to write to C:. Double check the drive letter.
    exit /b 1
)

echo.
echo === Building %BIN_NAME% for %TARGET% ===
pushd "%PROJECT_DIR%"
cargo build --release --target %TARGET%
set "BUILD_RESULT=%ERRORLEVEL%"
popd

if not "%BUILD_RESULT%"=="0" (
    echo.
    echo Build failed. Aborting deploy.
    exit /b 1
)

if not exist "%BUILD_OUT%" (
    echo.
    echo ERROR: Expected output not found at:
    echo   %BUILD_OUT%
    echo Check BIN_NAME / TARGET settings at the top of this script.
    exit /b 1
)

echo.
echo === Deploying to %DRIVE% ===
if not exist "%DRIVE%\EFI\BOOT" (
    mkdir "%DRIVE%\EFI\BOOT"
)

copy /Y "%BUILD_OUT%" "%DRIVE%\EFI\BOOT\BOOTX64.EFI" >nul
if not "%ERRORLEVEL%"=="0" (
    echo ERROR: Copy failed.
    exit /b 1
)

echo.
echo Done. %DRIVE%\EFI\BOOT\BOOTX64.EFI updated.
endlocal