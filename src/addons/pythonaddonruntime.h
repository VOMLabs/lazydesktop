#ifndef PYTHONADDONRUNTIME_H
#define PYTHONADDONRUNTIME_H

#include "addonruntime.h"
#include <QProcess>
#include <QJsonObject>
#include <QJsonArray>

class AddonAPI;
struct AddonManifest;

class PythonAddonRuntime : public AddonRuntime
{
    Q_OBJECT

public:
    PythonAddonRuntime(const AddonManifest &manifest, const QString &pluginDir,
                       AddonAPI *api, QObject *parent = nullptr);
    ~PythonAddonRuntime() override;

    bool load() override;
    bool enable() override;
    bool disable() override;
    bool unload() override;
    bool reload() override;

    bool dispatchEvent(AddonEvent event, const QJsonObject &data = {}) override;
    QJsonValue callFunction(const QString &name, const QJsonArray &args = {}) override;
    QJsonValue executeCommand(const QString &name, const QJsonArray &args = {}) override;

private slots:
    void onProcessReadyRead();
    void onProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onProcessErrorOccurred(QProcess::ProcessError error);

private:
    bool setupEnvironment();
    bool startProcess();
    void stopProcess();
    void sendMessage(const QJsonObject &msg);
    void handleMessage(const QJsonObject &msg);
    QString venvPath() const;
    QString pythonPath() const;

    AddonAPI *m_api = nullptr;
    QProcess *m_process = nullptr;
    QByteArray m_readBuffer;
    bool m_pythonAvailable = false;
    int m_messageId = 0;
    QString m_uvPath;
};

#endif
