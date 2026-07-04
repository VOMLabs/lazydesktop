#ifndef ADDONMANAGER_H
#define ADDONMANAGER_H

#include <QObject>
#include <QHash>
#include <QStringList>
#include <QJsonObject>
#include <QJsonArray>
#include <functional>

#include "addonruntime.h"

class AddonAPI;
class QNetworkAccessManager;
class QMenu;

class AddonManager : public QObject
{
    Q_OBJECT

public:
    explicit AddonManager(QNetworkAccessManager *network, QObject *parent = nullptr);
    ~AddonManager() override;

    static AddonManager *instance();
    static void initialize(QNetworkAccessManager *network);

    bool installFromArchive(const QString &archivePath);
    bool uninstall(const QString &pluginId);
    bool enable(const QString &pluginId);
    bool disable(const QString &pluginId);
    bool reload(const QString &pluginId);
    bool reloadAll();

    AddonRuntime *runtime(const QString &pluginId) const;
    AddonManifest manifest(const QString &pluginId) const;
    AddonStatus status(const QString &pluginId) const;
    AddonAPI *api(const QString &pluginId) const;

    QStringList installedPlugins() const;
    QStringList enabledPlugins() const;
    QStringList disabledPlugins() const;
    QStringList incompatiblePlugins() const;

    bool isInstalled(const QString &pluginId) const;
    bool isEnabled(const QString &pluginId) const;
    bool hasDuplicateId(const QString &pluginId) const;

    void scanInstalled();
    QString addonsDir() const;
    QString pluginDir(const QString &pluginId) const;

    void dispatchEventToAll(AddonEvent event, const QJsonObject &data = {});

    int pluginCount() const { return m_runtimes.size(); }
    qint64 pluginSize(const QString &pluginId) const;

    QNetworkAccessManager *networkManager() const { return m_network; }

    // --- Command registry ---
    using CommandCallback = std::function<QJsonValue(const QJsonArray &)>;
    bool registerCommand(const QString &pluginId, const QString &name, CommandCallback callback);
    void unregisterPluginCommands(const QString &pluginId);
    QJsonValue executeCommand(const QString &name, const QJsonArray &args = {});
    QStringList allCommands() const;
    QString commandOwner(const QString &name) const;

    // --- Menu contributions ---
    struct MenuContribution {
        QString pluginId;
        QString parentPath;
        QString label;
        QString commandId;
    };
    bool registerMenu(const QString &pluginId, const QString &parentPath,
                      const QString &label, const QString &commandId);
    QVector<MenuContribution> menuContributions() const { return m_menuContributions; }
    void unregisterPluginMenus(const QString &pluginId);

    // --- Rebuild the Addons menu in the given QMenu ---
    void populateAddonsMenu(QMenu *menu) const;

signals:
    void pluginInstalled(const QString &pluginId);
    void pluginUninstalled(const QString &pluginId);
    void pluginEnabled(const QString &pluginId);
    void pluginDisabled(const QString &pluginId);
    void pluginReloaded(const QString &pluginId);
    void pluginError(const QString &pluginId, const QString &error);
    void addonsChanged();

private:
    bool loadPlugin(const QString &pluginId);
    bool unloadPlugin(const QString &pluginId);
    bool validateAndInstall(const QString &extractPath, const AddonManifest &manifest);
    QString findManifestFile(const QString &dirPath) const;
    void saveInstalledList();
    void loadInstalledList();
    QString installedListPath() const;

    QHash<QString, AddonRuntime *> m_runtimes;
    QHash<QString, AddonAPI *> m_apis;
    QStringList m_installedIds;
    QNetworkAccessManager *m_network;

    // commandName -> (pluginId, callback)
    QHash<QString, QPair<QString, CommandCallback>> m_commands;
    QVector<MenuContribution> m_menuContributions;

    static AddonManager *s_instance;
};

#endif
