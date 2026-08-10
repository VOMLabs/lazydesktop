@echo off
REM Build LazyDesktop MSI Installer
REM Requires: Visual Studio, Qt 6, WiX Toolset v3+
REM
REM Prerequisites:
REM   1. Install Qt 6 for Windows (qt.io/download-open-source)
REM   2. Install WiX Toolset (wixtoolset.org)
REM   3. Open "x64 Native Tools Command Prompt for VS"
REM
REM Usage:
REM   set QT_DIR=C:\Qt\6.11.1\msvc2022_64
REM   build-msi.bat

setlocal enabledelayedexpansion

if "%QT_DIR%"=="" (
    echo Please set QT_DIR to your Qt installation path.
    echo Example: set QT_DIR=C:\Qt\6.11.1\msvc2022_64
    exit /b 1
)

set BUILD_DIR=%CD%\build-msi
set SOURCE_DIR=%CD%\..\..

echo === Configuring with Meson ===
meson setup "%BUILD_DIR%" "%SOURCE_DIR%" ^
    -Dbuildtype=release ^
    -Dwarning_level=0

if errorlevel 1 exit /b 1

echo === Building ===
ninja -C "%BUILD_DIR%"

if errorlevel 1 exit /b 1

echo === Generating MSI ===
candle.exe -arch x64 ^
    -dBuildDir="%BUILD_DIR%\src" ^
    -dQtDir="%QT_DIR%" ^
    -dSourceDir="%SOURCE_DIR%" ^
    lazydesktop.wxs

if errorlevel 1 exit /b 1

light.exe -ext WixUIExtension ^
    -cultures:en-US ^
    -out "lazydesktop-0.2.0.msi" ^
    lazydesktop.wixobj

if errorlevel 1 exit /b 1

echo === MSI built successfully: lazydesktop-0.2.0.msi ===
