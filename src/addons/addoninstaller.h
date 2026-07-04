#ifndef ADDONINSTALLER_H
#define ADDONINSTALLER_H

#include <QString>
#include <QObject>

#include "addonmanifest.h"

class AddonInstaller : public QObject
{

public:
    explicit AddonInstaller(QObject *parent = nullptr);

    struct InstallResult
    {
        bool success = false;
        QString errorMessage;
        QString pluginId;
        QString extractPath;
        AddonManifest manifest;
    };

    InstallResult install(const QString &archivePath, const QString &targetDir);
    bool uninstall(const QString &pluginDir);

    static bool isArchive(const QString &path);
    static bool isLzaArchive(const QString &path);
    static bool isZipArchive(const QString &path);

    static QString extractArchive(const QString &archivePath, const QString &targetDir);
    static bool validateStructure(const QString &extractDir);

private:
    static bool extractZip(const QString &zipPath, const QString &targetDir);
    static bool ensureDirectory(const QString &dir);
    QString findManifestFile(const QString &dirPath) const;
};

#endif
