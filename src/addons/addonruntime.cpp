#include "addonruntime.h"
#include "addonmanifest.h"
#include <QDir>
#include <QStandardPaths>

AddonRuntime::AddonRuntime(const AddonManifest &manifest, const QString &pluginDir, QObject *parent)
    : QObject(parent)
    , m_manifest(manifest)
    , m_pluginDir(pluginDir)
{
}

AddonRuntime::~AddonRuntime() = default;

QString AddonRuntime::statusString() const
{
    switch (m_status) {
    case AddonStatus::Installed:    return "Installed";
    case AddonStatus::Loading:      return "Loading...";
    case AddonStatus::Loaded:       return "Loaded";
    case AddonStatus::Enabled:      return "Enabled";
    case AddonStatus::Disabled:     return "Disabled";
    case AddonStatus::Error:        return "Error";
    case AddonStatus::Incompatible: return "Incompatible";
    }
    return "Unknown";
}

QString AddonRuntime::configDir() const
{
    return m_pluginDir + "/conf";
}

QString AddonRuntime::dataDir() const
{
    QString path = m_pluginDir + "/data";
    QDir().mkpath(path);
    return path;
}
