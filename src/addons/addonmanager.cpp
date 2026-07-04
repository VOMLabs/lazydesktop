#include "addonmanager.h"
#include "addonmanifest.h"
#include "addonruntime.h"
#include "addonapi.h"
#include "addoninstaller.h"
#include "pythonaddonruntime.h"
#include "addonpermissions.h"

#ifdef LUA_FOUND
#include "luaaddonruntime.h"
#endif

#include <QMenu>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkAccessManager>
#include <QApplication>

AddonManager *AddonManager::s_instance = nullptr;

AddonManager::AddonManager(QNetworkAccessManager *network, QObject *parent)
    : QObject(parent)
    , m_network(network)
{
    s_instance = this;

    QDir().mkpath(addonsDir());
    loadInstalledList();
    scanInstalled();
}

AddonManager::~AddonManager()
{
    for (auto *rt : m_runtimes) {
        rt->disable();
        rt->unload();
    }
    qDeleteAll(m_runtimes);
    qDeleteAll(m_apis);
    s_instance = nullptr;
}

AddonManager *AddonManager::instance()
{
    return s_instance;
}

void AddonManager::initialize(QNetworkAccessManager *network)
{
    new AddonManager(network, qApp);
}

bool AddonManager::installFromArchive(const QString &archivePath)
{
    AddonInstaller installer;
    auto result = installer.install(archivePath, addonsDir());

    if (!result.success) {
        emit pluginError(result.pluginId, result.errorMessage);
        return false;
    }

    if (isInstalled(result.pluginId)) {
        QDir(result.extractPath).removeRecursively();
        emit pluginError(result.pluginId, "Duplicate plugin ID: " + result.pluginId);
        return false;
    }

    if (!m_installedIds.contains(result.pluginId))
        m_installedIds.append(result.pluginId);
    saveInstalledList();

    if (!loadPlugin(result.pluginId)) {
        emit pluginError(result.pluginId, "Plugin installed but failed to load");
        return false;
    }

    emit pluginInstalled(result.pluginId);
    emit addonsChanged();
    return true;
}

bool AddonManager::uninstall(const QString &pluginId)
{
    if (!isInstalled(pluginId))
        return false;

    unloadPlugin(pluginId);

    QString dir = pluginDir(pluginId);

    AddonInstaller installer;
    installer.uninstall(dir);

    m_installedIds.removeAll(pluginId);
    saveInstalledList();

    emit pluginUninstalled(pluginId);
    emit addonsChanged();
    return true;
}

bool AddonManager::enable(const QString &pluginId)
{
    auto *rt = m_runtimes.value(pluginId);
    if (!rt)
        return false;
    return rt->enable();
}

bool AddonManager::disable(const QString &pluginId)
{
    auto *rt = m_runtimes.value(pluginId);
    if (!rt)
        return false;
    return rt->disable();
}

bool AddonManager::reload(const QString &pluginId)
{
    auto *rt = m_runtimes.value(pluginId);
    if (!rt)
        return false;
    bool ok = rt->reload();
    if (ok)
        emit pluginReloaded(pluginId);
    return ok;
}

bool AddonManager::reloadAll()
{
    bool allOk = true;
    for (auto it = m_runtimes.begin(); it != m_runtimes.end(); ++it) {
        if (!it.value()->reload())
            allOk = false;
    }
    return allOk;
}

AddonRuntime *AddonManager::runtime(const QString &pluginId) const
{
    return m_runtimes.value(pluginId);
}

AddonManifest AddonManager::manifest(const QString &pluginId) const
{
    auto *rt = m_runtimes.value(pluginId);
    if (rt)
        return rt->manifest();
    return {};
}

AddonStatus AddonManager::status(const QString &pluginId) const
{
    auto *rt = m_runtimes.value(pluginId);
    if (rt)
        return rt->status();
    return AddonStatus::Installed;
}

AddonAPI *AddonManager::api(const QString &pluginId) const
{
    return m_apis.value(pluginId);
}

QStringList AddonManager::installedPlugins() const
{
    return m_installedIds;
}

QStringList AddonManager::enabledPlugins() const
{
    QStringList list;
    for (auto it = m_runtimes.begin(); it != m_runtimes.end(); ++it) {
        if (it.value()->isEnabled())
            list.append(it.key());
    }
    return list;
}

QStringList AddonManager::disabledPlugins() const
{
    QStringList list;
    for (auto it = m_runtimes.begin(); it != m_runtimes.end(); ++it) {
        if (!it.value()->isEnabled() && it.value()->isLoaded())
            list.append(it.key());
    }
    return list;
}

QStringList AddonManager::incompatiblePlugins() const
{
    QStringList list;
    for (auto it = m_runtimes.begin(); it != m_runtimes.end(); ++it) {
        if (it.value()->status() == AddonStatus::Incompatible)
            list.append(it.key());
    }
    return list;
}

bool AddonManager::isInstalled(const QString &pluginId) const
{
    return m_runtimes.contains(pluginId) || m_installedIds.contains(pluginId);
}

bool AddonManager::isEnabled(const QString &pluginId) const
{
    auto *rt = m_runtimes.value(pluginId);
    return rt && rt->isEnabled();
}

bool AddonManager::hasDuplicateId(const QString &pluginId) const
{
    int count = 0;
    QDir dir(addonsDir());
    for (const auto &subdir : dir.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        AddonManifest m = AddonManifest::fromFile(dir.absoluteFilePath(subdir + "/manifest.yml"));
        if (!m.isValid)
            m = AddonManifest::fromFile(dir.absoluteFilePath(subdir + "/manifest.json"));
        if (m.isValid && m.id == pluginId)
            count++;
    }
    return count > 1;
}

void AddonManager::scanInstalled()
{
    QDir dir(addonsDir());
    for (const auto &subdir : dir.entryList(QDir::Dirs | QDir::NoDotAndDotDot)) {
        QString pluginPath = dir.absoluteFilePath(subdir);
        QString manifestPath = findManifestFile(pluginPath);
        if (manifestPath.isEmpty())
            continue;

        AddonManifest manifest = AddonManifest::fromFile(manifestPath);
        if (!manifest.isValid)
            continue;

        if (!m_installedIds.contains(manifest.id)) {
            m_installedIds.append(manifest.id);
        }

        loadPlugin(manifest.id);
    }
    saveInstalledList();
}

QString AddonManager::addonsDir() const
{
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
           + "/lazydesktop/addons";
}

QString AddonManager::pluginDir(const QString &pluginId) const
{
    return addonsDir() + "/" + pluginId;
}

void AddonManager::dispatchEventToAll(AddonEvent event, const QJsonObject &data)
{
    for (auto it = m_runtimes.begin(); it != m_runtimes.end(); ++it) {
        if (it.value()->isEnabled())
            it.value()->dispatchEvent(event, data);
    }
}

bool AddonManager::registerCommand(const QString &pluginId, const QString &name, CommandCallback callback)
{
    if (name.isEmpty() || !callback) return false;
    if (m_commands.contains(name)) {
        qDebug() << "[LazyAddons] Command already registered:" << name;
        return false;
    }
    m_commands[name] = {pluginId, callback};
    qDebug() << "[LazyAddons] Plugin" << pluginId << "registered command:" << name;
    return true;
}

void AddonManager::unregisterPluginCommands(const QString &pluginId)
{
    for (auto it = m_commands.begin(); it != m_commands.end();) {
        if (it.value().first == pluginId)
            it = m_commands.erase(it);
        else
            ++it;
    }
}

QJsonValue AddonManager::executeCommand(const QString &name, const QJsonArray &args)
{
    auto it = m_commands.find(name);
    if (it == m_commands.end()) {
        qDebug() << "[LazyAddons] Unknown command:" << name;
        return {};
    }
    const auto &[pluginId, callback] = it.value();
    auto *rt = m_runtimes.value(pluginId);
    if (!rt || !rt->isEnabled()) {
        qDebug() << "[LazyAddons] Plugin not enabled for command:" << name;
        return {};
    }
    return callback(args);
}

QStringList AddonManager::allCommands() const
{
    return m_commands.keys();
}

QString AddonManager::commandOwner(const QString &name) const
{
    auto it = m_commands.find(name);
    return it != m_commands.end() ? it.value().first : QString{};
}

bool AddonManager::registerMenu(const QString &pluginId, const QString &parentPath,
                                 const QString &label, const QString &commandId)
{
    if (label.isEmpty() || commandId.isEmpty()) return false;
    m_menuContributions.append({pluginId, parentPath, label, commandId});
    emit addonsChanged();
    return true;
}

void AddonManager::unregisterPluginMenus(const QString &pluginId)
{
    m_menuContributions.erase(
        std::remove_if(m_menuContributions.begin(), m_menuContributions.end(),
            [&](const MenuContribution &m) { return m.pluginId == pluginId; }),
        m_menuContributions.end());
}

void AddonManager::populateAddonsMenu(QMenu *menu) const
{
    for (const auto &mc : m_menuContributions) {
        auto *action = menu->addAction(mc.label);
        action->setData(mc.commandId);
    }
}

qint64 AddonManager::pluginSize(const QString &pluginId) const
{
    QString dir = pluginDir(pluginId);
    QDirIterator it(dir, QDir::AllEntries | QDir::NoDotAndDotDot, QDirIterator::Subdirectories);
    qint64 total = 0;
    while (it.hasNext()) {
        it.next();
        total += it.fileInfo().size();
    }
    return total;
}

bool AddonManager::loadPlugin(const QString &pluginId)
{
    if (m_runtimes.contains(pluginId))
        return true;

    QString pDir = pluginDir(pluginId);
    QString manifestPath = findManifestFile(pDir);
    if (manifestPath.isEmpty())
        return false;

    AddonManifest manifest = AddonManifest::fromFile(manifestPath);
    if (!manifest.isValid)
        return false;

    AddonRuntime *rt = nullptr;
    if (manifest.runtime == "lua") {
#ifdef LUA_FOUND
        rt = new LuaAddonRuntime(manifest, pDir, nullptr, this);
#else
        qDebug() << "[LazyAddons] Lua runtime not available; cannot load plugin:" << pluginId;
        return false;
#endif
    } else if (manifest.runtime == "python") {
        rt = new PythonAddonRuntime(manifest, pDir, nullptr, this);
    }

    if (!rt) {
        return false;
    }

    auto *addonApi = new AddonAPI(rt, m_network, rt);
    m_apis[pluginId] = addonApi;
    rt->setAPI(addonApi);

    connect(rt, &AddonRuntime::logMessage, this, [this, pluginId](const QString &msg, const QString &level) {
        qDebug() << "[LazyAddons:" << level << "]" << pluginId << ":" << msg;
    });

    connect(rt, &AddonRuntime::errorOccurred, this, [this, pluginId](const QString &err) {
        emit pluginError(pluginId, err);
    });

    m_runtimes[pluginId] = rt;

    if (!rt->load()) {
        qDebug() << "[LazyAddons] Failed to load plugin:" << pluginId << rt->lastError();
        return false;
    }

    return true;
}

bool AddonManager::unloadPlugin(const QString &pluginId)
{
    unregisterPluginCommands(pluginId);
    unregisterPluginMenus(pluginId);

    auto *rt = m_runtimes.take(pluginId);
    if (!rt)
        return false;

    rt->disable();
    rt->unload();

    if (auto *api = m_apis.take(pluginId))
        delete api;

    delete rt;
    return true;
}

QString AddonManager::findManifestFile(const QString &dirPath) const
{
    for (const auto &name : {"manifest.yml", "manifest.yaml", "manifest.json"}) {
        QString path = dirPath + "/" + name;
        if (QFileInfo::exists(path))
            return path;
    }
    return {};
}

void AddonManager::saveInstalledList()
{
    QJsonObject root;
    QJsonArray plugins;
    for (const auto &id : m_installedIds)
        plugins.append(id);
    root["plugins"] = plugins;

    QString path = installedListPath();
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (file.open(QIODevice::WriteOnly)) {
        file.write(QJsonDocument(root).toJson());
        file.close();
    }
}

void AddonManager::loadInstalledList()
{
    QString path = installedListPath();
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly))
        return;

    QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
    file.close();

    if (!doc.isObject()) return;
    QJsonArray plugins = doc.object()["plugins"].toArray();
    m_installedIds.clear();
    for (const auto &p : plugins)
        m_installedIds.append(p.toString());
}

QString AddonManager::installedListPath() const
{
    return addonsDir() + "/installed.json";
}
