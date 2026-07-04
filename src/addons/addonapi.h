#ifndef ADDONAPI_H
#define ADDONAPI_H

#include <QObject>
#include <QJsonObject>
#include <QJsonArray>
#include <functional>

class AddonRuntime;
class AddonManager;
class QNetworkAccessManager;

class AddonAPI : public QObject
{
    Q_OBJECT

public:
    explicit AddonAPI(AddonRuntime *runtime = nullptr, QNetworkAccessManager *network = nullptr, QObject *parent = nullptr);
    void setRuntime(AddonRuntime *runtime) { m_runtime = runtime; }

    void log(const QString &message, const QString &level = "info");
    void logInfo(const QString &message);
    void logWarn(const QString &message);
    void logError(const QString &message);
    void logDebug(const QString &message);

    QString readConfig(const QString &key, const QString &defaultValue = {}) const;
    void writeConfig(const QString &key, const QString &value);
    QJsonObject allConfig() const;
    void saveConfig();

    QString dataPath(const QString &subpath = {}) const;
    QString assetPath(const QString &relativePath) const;
    QByteArray readAsset(const QString &relativePath) const;

    QString pluginDir() const;
    QString pluginId() const;
    QString pluginVersion() const;

    QNetworkAccessManager *network() const { return m_network; }

    QJsonObject defaultConfig() const;

    // --- Command registration ---
    bool registerCommand(const QString &name, std::function<QJsonValue(const QJsonArray &)> callback);

    // --- File CRUD (within plugin data dir, sandboxed) ---
    bool createFile(const QString &relativePath, const QByteArray &content);
    QByteArray readFileData(const QString &relativePath) const;
    bool writeFile(const QString &relativePath, const QByteArray &content);
    bool deleteFileData(const QString &relativePath);
    QStringList listFiles(const QString &relativeDir = {}) const;

    // --- Menu contributions ---
    bool createMenu(const QString &parentPath, const QString &label, const QString &commandId);

signals:
    void notify(const QString &title, const QString &message);

private:
    void loadDefaultConfig() const;

    AddonRuntime *m_runtime;
    QNetworkAccessManager *m_network;
    mutable QJsonObject m_defaultConfig;
    mutable bool m_defaultConfigLoaded = false;
};

#endif
