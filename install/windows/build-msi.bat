@echo off
REM Build LazyDesktop MSI Installer
REM Requires: Visual Studio Build Tools (MSVC), Rust, WiX Toolset v3+
REM
REM Prerequisites:
REM   1. Install Rust (rustup.rs) with the MSVC toolchain
REM   2. Install WiX Toolset (wixtoolset.org)
REM   3. Open "x64 Native Tools Command Prompt for VS"
REM
REM Usage:
REM   build-msi.bat

setlocal enabledelayedexpansion

set BUILD_DIR=%CD%\target\release
set SOURCE_DIR=%CD%\..\..

echo === Building Rust release binary ===
cargo build --release -p lazydesktop-app

if errorlevel 1 exit /b 1

echo === Generating MSI fragment from release dir ===
powershell -ExecutionPolicy Bypass -File "%CD%\deploy-msi.ps1" ^
    -BuildDir "%BUILD_DIR%" ^
    -OutputFile "%BUILD_DIR%\AppFiles.g.wxs"

if errorlevel 1 exit /b 1

echo === Building MSI ===
candle.exe -arch x64 ^
    -dBuildDir="%BUILD_DIR%" ^
    -dSourceDir="%SOURCE_DIR%" ^
    "%BUILD_DIR%\AppFiles.g.wxs" ^
    "lazydesktop.wxs"

if errorlevel 1 exit /b 1

light.exe -ext WixUIExtension ^
    -cultures:en-US ^
    -out "lazydesktop-0.3.0.msi" ^
    lazydesktop.wixobj AppFiles.g.wixobj

if errorlevel 1 exit /b 1

echo === MSI built successfully: lazydesktop-0.3.0.msi ===