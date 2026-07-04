#ifndef ADDONRUNTIME_H
#define ADDONRUNTIME_H

#include <QObject>
#include <QString>
#include <QJsonObject>
#include <QJsonValue>
#include <QJsonArray>
#include <functional>

#include "addonmanifest.h"

class AddonManager;
class AddonAPI;

enum class AddonStatus
{
    Installed,
    Loading,
    Loaded,
    Enabled,
    Disabled,
    Error,
    Incompatible
};

enum class AddonEvent
{
    Load,
    Enable,
    Disable,
    Unload,
    SettingsChanged,
    AppStarted,
    AppShuttingDown,
    ProjectOpened,
    ProjectClosed,
    ThemeChanged,
    FileOpened,
    FileSaved,
    WindowCreated,
    WindowClosed
};

class AddonRuntime : public QObject
{
    Q_OBJECT

public:
    explicit AddonRuntime(const AddonManifest &manifest, const QString &pluginDir, QObject *parent = nullptr);
    ~AddonRuntime() override;

    virtual bool load() = 0;
    virtual bool enable() = 0;
    virtual bool disable() = 0;
    virtual bool unload() = 0;
    virtual bool reload() = 0;

    virtual bool dispatchEvent(AddonEvent event, const QJsonObject &data = {}) = 0;
    virtual QJsonValue callFunction(const QString &name, const QJsonArray &args = {}) = 0;
    virtual QJsonValue executeCommand(const QString &name, const QJsonArray &args = {}) = 0;

    AddonStatus status() const { return m_status; }
    QString statusString() const;
    QString lastError() const { return m_lastError; }

    AddonManifest manifest() const { return m_manifest; }
    QString pluginDir() const { return m_pluginDir; }
    QString configDir() const;
    QString dataDir() const;

    bool isLoaded() const { return m_status == AddonStatus::Loaded || m_status == AddonStatus::Enabled; }
    bool isEnabled() const { return m_status == AddonStatus::Enabled; }

    void setAPI(AddonAPI *api) { m_api = api; }
    AddonAPI *api() const { return m_api; }

signals:
    void statusChanged(AddonStatus newStatus);
    void errorOccurred(const QString &error);
    void logMessage(const QString &message, const QString &level);

protected:
    AddonManifest m_manifest;
    QString m_pluginDir;
    AddonStatus m_status = AddonStatus::Installed;
    QString m_lastError;
    AddonAPI *m_api = nullptr;
};

#endif
