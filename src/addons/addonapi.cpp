#include "addonapi.h"
#include "addonruntime.h"
#include "addonmanifest.h"
#include "addonmanager.h"

#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QStandardPaths>
#include <QFileInfo>
#include <QNetworkAccessManager>
#include <yaml-cpp/yaml.h>

AddonAPI::AddonAPI(AddonRuntime *runtime, QNetworkAccessManager *network, QObject *parent)
    : QObject(parent)
    , m_runtime(runtime)
    , m_network(network)
{
}

void AddonAPI::log(const QString &message, const QString &level)
{
    emit m_runtime->logMessage(message, level);
}

void AddonAPI::logInfo(const QString &message) { log(message, "info"); }
void AddonAPI::logWarn(const QString &message) { log(message, "warn"); }
void AddonAPI::logError(const QString &message) { log(message, "error"); }
void AddonAPI::logDebug(const QString &message) { log(message, "debug"); }

QString AddonAPI::readConfig(const QString &key, const QString &defaultValue) const
{
    QDir().mkpath(m_runtime->configDir());

    QFile file(m_runtime->configDir() + "/settings.json");
    if (file.open(QIODevice::ReadOnly)) {
        QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
        file.close();
        if (doc.isObject() && doc.object().contains(key))
            return doc.object().value(key).toString();
    }

    loadDefaultConfig();
    if (m_defaultConfig.contains(key))
        return m_defaultConfig.value(key).toString();

    return defaultValue;
}

void AddonAPI::writeConfig(const QString &key, const QString &value)
{
    QString path = m_runtime->configDir() + "/settings.json";
    QDir().mkpath(QFileInfo(path).absolutePath());

    QJsonObject obj;
    QFile file(path);
    if (file.open(QIODevice::ReadOnly)) {
        QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
        file.close();
        if (doc.isObject())
            obj = doc.object();
    }

    obj[key] = value;

    if (file.open(QIODevice::WriteOnly)) {
        file.write(QJsonDocument(obj).toJson());
        file.close();
    }

    if (m_runtime) {
        AddonManager *mgr = AddonManager::instance();
        if (mgr)
            mgr->dispatchEventToAll(AddonEvent::SettingsChanged);
    }
}

QJsonObject AddonAPI::allConfig() const
{
    QFile file(m_runtime->configDir() + "/settings.json");
    if (!file.open(QIODevice::ReadOnly))
        return {};

    QJsonDocument doc = QJsonDocument::fromJson(file.readAll());
    return doc.isObject() ? doc.object() : QJsonObject{};
}

void AddonAPI::saveConfig()
{
}

QString AddonAPI::dataPath(const QString &subpath) const
{
    QString base = m_runtime->dataDir();
    if (subpath.isEmpty())
        return base;
    return base + "/" + subpath;
}

QString AddonAPI::assetPath(const QString &relativePath) const
{
    return m_runtime->pluginDir() + "/assets/" + relativePath;
}

QByteArray AddonAPI::readAsset(const QString &relativePath) const
{
    QFile file(assetPath(relativePath));
    if (!file.open(QIODevice::ReadOnly))
        return {};
    return file.readAll();
}

QString AddonAPI::pluginDir() const { return m_runtime->pluginDir(); }
QString AddonAPI::pluginId() const { return m_runtime->manifest().id; }
QString AddonAPI::pluginVersion() const { return m_runtime->manifest().version; }

QJsonObject AddonAPI::defaultConfig() const
{
    loadDefaultConfig();
    return m_defaultConfig;
}

void AddonAPI::loadDefaultConfig() const
{
    if (m_defaultConfigLoaded)
        return;
    m_defaultConfigLoaded = true;

    QString confPath = m_runtime->pluginDir() + "/conf.yml";
    QFile file(confPath);
    if (!file.open(QIODevice::ReadOnly))
        return;

    QByteArray data = file.readAll();
    file.close();

    try {
        YAML::Node root = YAML::Load(data.toStdString());
        if (!root.IsMap())
            return;

        for (auto it = root.begin(); it != root.end(); ++it) {
            QString key = QString::fromStdString(it->first.as<std::string>());
            YAML::Node val = it->second;
            if (val.IsScalar())
                m_defaultConfig[key] = QString::fromStdString(val.Scalar());
            else if (val.IsSequence()) {
                QJsonArray arr;
                for (const auto &elem : val)
                    arr.append(QString::fromStdString(elem.Scalar()));
                m_defaultConfig[key] = arr;
            }
        }
    } catch (const YAML::Exception &) {
    }
}

bool AddonAPI::registerCommand(const QString &name, std::function<QJsonValue(const QJsonArray &)> callback)
{
    auto *mgr = AddonManager::instance();
    if (!mgr || !m_runtime) return false;
    return mgr->registerCommand(m_runtime->manifest().id, name, std::move(callback));
}

static QString sanitizeRelPath(const QString &rel)
{
    QString clean = rel;
    clean.replace("..", "");
    while (clean.startsWith('/')) clean.remove(0, 1);
    return clean;
}

bool AddonAPI::createFile(const QString &relativePath, const QByteArray &content)
{
    QString path = dataPath(sanitizeRelPath(relativePath));
    QDir().mkpath(QFileInfo(path).absolutePath());
    QFile file(path);
    if (!file.open(QIODevice::WriteOnly))
        return false;
    file.write(content);
    file.close();
    return true;
}

QByteArray AddonAPI::readFileData(const QString &relativePath) const
{
    QFile file(dataPath(sanitizeRelPath(relativePath)));
    if (!file.open(QIODevice::ReadOnly))
        return {};
    QByteArray data = file.readAll();
    file.close();
    return data;
}

bool AddonAPI::writeFile(const QString &relativePath, const QByteArray &content)
{
    return createFile(relativePath, content);
}

bool AddonAPI::deleteFileData(const QString &relativePath)
{
    QString path = dataPath(sanitizeRelPath(relativePath));
    return QFile::remove(path);
}

QStringList AddonAPI::listFiles(const QString &relativeDir) const
{
    QDir dir(dataPath(sanitizeRelPath(relativeDir)));
    if (!dir.exists()) return {};
    QStringList entries;
    for (const auto &info : dir.entryInfoList(QDir::Files | QDir::Dirs | QDir::NoDotAndDotDot))
        entries.append(info.fileName());
    return entries;
}

bool AddonAPI::createMenu(const QString &parentPath, const QString &label, const QString &commandId)
{
    auto *mgr = AddonManager::instance();
    if (!mgr || !m_runtime) return false;
    return mgr->registerMenu(m_runtime->manifest().id, parentPath, label, commandId);
}
