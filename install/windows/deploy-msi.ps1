# Deploy LazyDesktop runtime files next to the built exe and
# generate a WiX v4 fragment (.wxs) that installs every deployed file into
# INSTALLDIR. The fragment is referenced by lazydesktop.wxs via a
# ComponentGroup named "LazyDesktopBinaries".
#
# Usage:
#   powershell -File deploy-msi.ps1 -BuildDir <abs path to release bin dir> `
#       -OutputFile <path to .wxs fragment>
#
# WiX v4 removed the standalone heat/harvest tool from the core CLI, so we
# generate the file components from the deployed directory ourselves.

param(
    [Parameter(Mandatory = $true)]
    [string]$BuildDir,

    [Parameter(Mandatory = $true)]
    [string]$OutputFile,

    # Kept for compatibility with older invocations; unused (no Qt runtime).
    [string]$QtRoot = ""
)

$ErrorActionPreference = "Stop"

# ---------------------------------------------------------------------------
# 1. Make sure the built exe is present
# ---------------------------------------------------------------------------
$exe = Join-Path $BuildDir "lazydesktop.exe"
if (-not (Test-Path $exe)) {
    throw "Built executable not found at: $exe"
}

# ---------------------------------------------------------------------------
# 2. Walk the release directory tree and emit a WiX v4 fragment
# ---------------------------------------------------------------------------
$root = (Resolve-Path $BuildDir).Path.TrimEnd('\')
$files = Get-ChildItem -Path $root -Recurse -File | Sort-Object FullName

$sb = [System.Text.StringBuilder]::new()
[void]$sb.AppendLine('<?xml version="1.0" encoding="UTF-8"?>')
[void]$sb.AppendLine('<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">')
[void]$sb.AppendLine('  <Fragment>')
[void]$sb.AppendLine('    <DirectoryRef Id="INSTALLDIR">')

# Collect all files (recursive) so we can build the directory hierarchy.
$files = Get-ChildItem -Path $root -Recurse -File | Sort-Object FullName

# Build the set of relative directories first.
$dirSet = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
foreach ($f in $files) {
    $relDir = $f.DirectoryName.Substring($root.Length).TrimStart('\')
    if ($relDir -eq '') { continue }
    $parts = $relDir -split '\\'
    $cur = ''
    foreach ($part in $parts) {
        $cur = if ($cur -eq '') { $part } else { "$cur\$part" }
        [void]$dirSet.Add($cur)
    }
}

# Sort directories so parents come before children (path depth order).
$dirs = @($dirSet) | Sort-Object { ($_ -split '\\').Count }, { $_ }

# Map relative dir path -> WiX Directory Id. Directories are emitted as flat
# siblings under INSTALLDIR (each `<Directory>` under a DirectoryRef is a
# child of that ref, which is exactly what we want); components reference them
# by Id, so nesting here is unnecessary.
$dirIdMap = @{}
$dirIndex = 0
foreach ($d in $dirs) {
    $id = "dir_$dirIndex"
    $dirIdMap[$d] = $id
    $dirIndex++
    $name = $d.Split('\')[-1]
    [void]$sb.AppendLine("      <Directory Id=""$id"" Name=""$name""/>")
}

[void]$sb.AppendLine('    </DirectoryRef>')
[void]$sb.AppendLine('  </Fragment>')
[void]$sb.AppendLine('')

# ---------------------------------------------------------------------------
# 3. Emit a ComponentGroup with one component per file.
# ---------------------------------------------------------------------------
[void]$sb.AppendLine('  <Fragment>')
[void]$sb.AppendLine('    <ComponentGroup Id="LazyDesktopBinaries">')

$compIndex = 0
foreach ($f in $files) {
    $relPath = $f.FullName.Substring($root.Length).TrimStart('\')
    $relDir = $f.DirectoryName.Substring($root.Length).TrimStart('\')

    $compId = "comp_$compIndex"
    $fileId = "file_$compIndex"
    $compIndex++

    $name = $f.Name
    $source = $f.FullName

    $isExe = ($relPath -ieq "lazydesktop.exe")

    if ($isExe) {
        [void]$sb.AppendLine("      <Component Id=""$compId"" Bitness=""always64"">")
        [void]$sb.AppendLine("        <File Id=""$fileId"" Name=""$name"" Source=""$source"" KeyPath=""yes"" Vital=""yes""/>")
        [void]$sb.AppendLine("        <Environment Id=""PATH"" Name=""PATH"" Value=""[INSTALLDIR]"" Permanent=""no"" Part=""last"" Action=""set"" System=""yes""/>")
        [void]$sb.AppendLine("      </Component>")
    }
    else {
        $dirId = if ($relDir -eq '') { $null } else { $dirIdMap[$relDir] }
        if ($dirId) {
            [void]$sb.AppendLine("      <Component Id=""$compId"" Directory=""$dirId"" Bitness=""always64"">")
        }
        else {
            [void]$sb.AppendLine("      <Component Id=""$compId"" Bitness=""always64"">")
        }
        [void]$sb.AppendLine("        <File Id=""$fileId"" Name=""$name"" Source=""$source"" KeyPath=""yes""/>")
        [void]$sb.AppendLine("      </Component>")
    }
}

[void]$sb.AppendLine('    </ComponentGroup>')
[void]$sb.AppendLine('  </Fragment>')
[void]$sb.AppendLine('</Wix>')

$dir = Split-Path -Parent $OutputFile
if (-not (Test-Path $dir)) {
    New-Item -ItemType Directory -Path $dir -Force | Out-Null
}
$sb.ToString() | Set-Content -Path $OutputFile -Encoding UTF8

Write-Host "Generated WiX fragment: $OutputFile ($($files.Count) files)"
