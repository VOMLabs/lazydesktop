#include "addoninstaller.h"
#include "addonmanifest.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcess>
#include <QRegularExpression>
#include <QTemporaryDir>
#include <QDirIterator>

AddonInstaller::AddonInstaller(QObject *parent)
    : QObject(parent)
{
}

AddonInstaller::InstallResult AddonInstaller::install(const QString &archivePath, const QString &targetDir)
{
    InstallResult result;

    if (!isArchive(archivePath)) {
        result.success = false;
        result.errorMessage = "Not a valid archive file: " + archivePath;
        return result;
    }

    QString extractDir = extractArchive(archivePath, targetDir);
    if (extractDir.isEmpty()) {
        result.success = false;
        result.errorMessage = "Failed to extract archive";
        return result;
    }

    if (!validateStructure(extractDir)) {
        QDir(extractDir).removeRecursively();
        result.success = false;
        result.errorMessage = "Invalid plugin structure: missing manifest.yml or manifest.json";
        return result;
    }

    QString manifestPath = findManifestFile(extractDir);
    AddonManifest manifest = AddonManifest::fromFile(manifestPath);

    if (!manifest.isValid) {
        QDir(extractDir).removeRecursively();
        result.success = false;
        result.errorMessage = "Invalid manifest: " + manifest.errorMessage;
        return result;
    }

    result.success = true;
    result.pluginId = manifest.id;
    result.extractPath = extractDir;
    result.manifest = manifest;
    return result;
}

bool AddonInstaller::uninstall(const QString &pluginDir)
{
    QDir dir(pluginDir);
    if (!dir.exists())
        return false;
    return dir.removeRecursively();
}

bool AddonInstaller::isArchive(const QString &path)
{
    return isLzaArchive(path) || isZipArchive(path);
}

bool AddonInstaller::isLzaArchive(const QString &path)
{
    return path.endsWith(".lza", Qt::CaseInsensitive);
}

bool AddonInstaller::isZipArchive(const QString &path)
{
    return path.endsWith(".zip", Qt::CaseInsensitive);
}

QString AddonInstaller::extractArchive(const QString &archivePath, const QString &targetDir)
{
    if (!extractZip(archivePath, targetDir))
        return {};
    return targetDir;
}

bool AddonInstaller::validateStructure(const QString &extractDir)
{
    QDir dir(extractDir);
    QStringList entries = dir.entryList(QDir::Files | QDir::NoDotAndDotDot);

    for (const auto &manifest : {"manifest.yml", "manifest.yaml", "manifest.json"}) {
        if (QFileInfo::exists(extractDir + "/" + manifest))
            return true;
    }

    QDirIterator it(extractDir, {"manifest.yml", "manifest.yaml", "manifest.json"},
                    QDir::Files, QDirIterator::Subdirectories);
    return it.hasNext();
}

QString AddonInstaller::findManifestFile(const QString &dirPath) const
{
    for (const auto &name : {"manifest.yml", "manifest.yaml", "manifest.json"}) {
        QString path = dirPath + "/" + name;
        if (QFileInfo::exists(path))
            return path;
    }
    return {};
}

bool AddonInstaller::extractZip(const QString &zipPath, const QString &targetDir)
{
    QDir().mkpath(targetDir);

    auto tryLibzip = [&]() -> bool {
        QProcess proc;
        QStringList args;
#ifdef Q_OS_WIN
        args << "x" << zipPath << "-o" << "-y" << "-o" + targetDir;
        proc.start("tar", args);
#else
        args << "-o" << zipPath << "-d" << targetDir;
        proc.start("unzip", args);
#endif
        proc.waitForFinished(30000);
        return proc.exitCode() == 0;
    };

    auto tryQt = [&]() -> bool {
        QProcess proc;
        proc.setWorkingDirectory(targetDir);
#ifdef Q_OS_WIN
        proc.start("powershell", {
            "-NoProfile", "-Command",
            "Expand-Archive -Path '" + zipPath + "' -DestinationPath '" + targetDir + "' -Force"
        });
#else
        proc.start("python3", {"-c", R"(
import sys, zipfile, os
zp = sys.argv[1]
td = sys.argv[2]
os.makedirs(td, exist_ok=True)
with zipfile.ZipFile(zp, 'r') as z:
    z.extractall(td)
)" , zipPath, targetDir});
#endif
        proc.waitForFinished(30000);
        return proc.exitCode() == 0;
    };

    if (tryLibzip())
        return true;
    if (tryQt())
        return true;

    return false;
}

bool AddonInstaller::ensureDirectory(const QString &dir)
{
    return QDir().mkpath(dir);
}
