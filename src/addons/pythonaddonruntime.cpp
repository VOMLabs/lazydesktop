#include "pythonaddonruntime.h"
#include "addonmanifest.h"
#include "addonapi.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QProcess>
#include <QTimer>
#include <QStandardPaths>

PythonAddonRuntime::PythonAddonRuntime(const AddonManifest &manifest, const QString &pluginDir,
                                       AddonAPI *api, QObject *parent)
    : AddonRuntime(manifest, pluginDir, parent)
    , m_api(api)
{
}

PythonAddonRuntime::~PythonAddonRuntime()
{
    unload();
}

bool PythonAddonRuntime::load()
{
    if (m_process) return true;

    m_status = AddonStatus::Loading;

    if (!setupEnvironment()) {
        m_status = AddonStatus::Error;
        return false;
    }

    if (!startProcess()) {
        m_status = AddonStatus::Error;
        return false;
    }

    QJsonObject loadMsg;
    loadMsg["method"] = "onLoad";
    loadMsg["id"] = ++m_messageId;
    sendMessage(loadMsg);

    m_status = AddonStatus::Loaded;
    return true;
}

bool PythonAddonRuntime::enable()
{
    if (m_status == AddonStatus::Enabled) return true;
    if (m_status != AddonStatus::Loaded && !load())
        return false;

    QJsonObject msg;
    msg["method"] = "onEnable";
    msg["id"] = ++m_messageId;
    sendMessage(msg);

    m_status = AddonStatus::Enabled;
    emit statusChanged(m_status);
    return true;
}

bool PythonAddonRuntime::disable()
{
    if (m_status != AddonStatus::Enabled) return true;

    QJsonObject msg;
    msg["method"] = "onDisable";
    msg["id"] = ++m_messageId;
    sendMessage(msg);

    m_status = AddonStatus::Loaded;
    emit statusChanged(m_status);
    return true;
}

bool PythonAddonRuntime::unload()
{
    if (!m_process) return true;

    QJsonObject msg;
    msg["method"] = "onUnload";
    msg["id"] = ++m_messageId;
    sendMessage(msg);

    stopProcess();
    m_status = AddonStatus::Installed;
    emit statusChanged(m_status);
    return true;
}

bool PythonAddonRuntime::reload()
{
    bool wasEnabled = (m_status == AddonStatus::Enabled);
    unload();
    if (!load())
        return false;
    if (wasEnabled && !enable())
        return false;
    emit statusChanged(m_status);
    return true;
}

bool PythonAddonRuntime::dispatchEvent(AddonEvent event, const QJsonObject &data)
{
    static const QHash<AddonEvent, QString> eventNames = {
        {AddonEvent::AppStarted,       "onAppStarted"},
        {AddonEvent::AppShuttingDown,  "onAppShuttingDown"},
        {AddonEvent::ProjectOpened,    "onProjectOpened"},
        {AddonEvent::ProjectClosed,    "onProjectClosed"},
        {AddonEvent::ThemeChanged,     "onThemeChanged"},
        {AddonEvent::FileOpened,       "onFileOpened"},
        {AddonEvent::FileSaved,        "onFileSaved"},
        {AddonEvent::WindowCreated,    "onWindowCreated"},
        {AddonEvent::WindowClosed,     "onWindowClosed"},
        {AddonEvent::SettingsChanged,  "onSettingsChanged"},
    };

    if (m_process && m_process->state() == QProcess::Running) {
        QJsonObject msg;
        msg["method"] = eventNames.value(event);
        msg["params"] = QJsonObject{{"data", data}};
        msg["id"] = ++m_messageId;
        sendMessage(msg);
    }

    return true;
}

QJsonValue PythonAddonRuntime::callFunction(const QString &name, const QJsonArray &args)
{
    if (!m_process || m_process->state() != QProcess::Running)
        return {};

    QJsonObject msg;
    msg["method"] = name;
    msg["params"] = QJsonObject{{"args", args}};
    msg["id"] = ++m_messageId;
    sendMessage(msg);

    return {};
}

QJsonValue PythonAddonRuntime::executeCommand(const QString &name, const QJsonArray &args)
{
    if (!m_process || m_process->state() != QProcess::Running)
        return {};

    QJsonObject msg;
    msg["method"] = "execute_command";
    msg["params"] = QJsonObject{{"name", name}, {"args", args}};
    msg["id"] = ++m_messageId;
    sendMessage(msg);

    return {};
}

void PythonAddonRuntime::onProcessReadyRead()
{
    m_readBuffer.append(m_process->readAllStandardOutput());

    while (true) {
        int nlIdx = m_readBuffer.indexOf('\n');
        if (nlIdx < 0) break;

        QByteArray line = m_readBuffer.left(nlIdx).trimmed();
        m_readBuffer.remove(0, nlIdx + 1);

        if (line.isEmpty()) continue;

        QJsonParseError err;
        QJsonDocument doc = QJsonDocument::fromJson(line, &err);
        if (err.error != QJsonParseError::NoError) {
            qDebug() << "[LazyAddons] Bad JSON from Python plugin" << m_manifest.id << err.errorString();
            continue;
        }

        if (doc.isObject())
            handleMessage(doc.object());
    }

    QByteArray errData = m_process->readAllStandardError();
    if (!errData.isEmpty()) {
        emit logMessage(QString::fromUtf8(errData).trimmed(), "error");
    }
}

void PythonAddonRuntime::onProcessFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus == QProcess::CrashExit) {
        m_lastError = "Python process crashed";
        m_status = AddonStatus::Error;
        emit errorOccurred(m_lastError);
    } else if (exitCode != 0) {
        QString stderr = QString::fromUtf8(m_process ? m_process->readAllStandardError() : QByteArray());
        m_lastError = "Python process exited with code " + QString::number(exitCode)
                      + (!stderr.isEmpty() ? ": " + stderr.trimmed() : "");
        m_status = AddonStatus::Error;
        emit errorOccurred(m_lastError);
    }

    if (m_process) {
        m_process->deleteLater();
        m_process = nullptr;
    }
}

void PythonAddonRuntime::onProcessErrorOccurred(QProcess::ProcessError error)
{
    switch (error) {
    case QProcess::FailedToStart:
        m_lastError = "Failed to start Python process";
        break;
    case QProcess::Crashed:
        m_lastError = "Python process crashed";
        break;
    case QProcess::Timedout:
        m_lastError = "Python process timed out";
        break;
    default:
        m_lastError = "Python process error";
        break;
    }
    m_status = AddonStatus::Error;
    emit errorOccurred(m_lastError);

    if (m_process) {
        m_process->deleteLater();
        m_process = nullptr;
    }
}

bool PythonAddonRuntime::setupEnvironment()
{
    m_uvPath = QStandardPaths::findExecutable("uv");
    if (m_uvPath.isEmpty()) {
        m_lastError = "uv not found. Python plugins require uv: https://docs.astral.sh/uv/";
        return false;
    }

    QString pyprojectPath = m_pluginDir + "/pyproject.toml";
    if (!QFileInfo::exists(pyprojectPath)) {
        m_lastError = "Python plugin missing pyproject.toml";
        return false;
    }

    QString venvDir = venvPath();
    if (!QDir(venvDir).exists()) {
        emit logMessage("Creating Python virtual environment for " + m_manifest.id + "...", "info");

        QProcess uvSync;
        uvSync.setWorkingDirectory(m_pluginDir);
        uvSync.start(m_uvPath, {"sync", "--frozen"});
        uvSync.waitForFinished(120000);

        if (uvSync.exitCode() != 0) {
            m_lastError = "uv sync failed: " + QString::fromUtf8(uvSync.readAllStandardError());
            return false;
        }

        emit logMessage("Python environment created for " + m_manifest.id, "info");
    }

    QString py = pythonPath();
    if (!QFileInfo::exists(py)) {
        m_lastError = "Python executable not found in venv: " + py;
        return false;
    }

    m_pythonAvailable = true;
    return true;
}

bool PythonAddonRuntime::startProcess()
{
    if (!m_pythonAvailable) return false;

    stopProcess();

    m_process = new QProcess(this);
    connect(m_process, &QProcess::readyReadStandardOutput, this, &PythonAddonRuntime::onProcessReadyRead);
    connect(m_process, QOverload<int, QProcess::ExitStatus>::of(&QProcess::finished),
            this, &PythonAddonRuntime::onProcessFinished);
    connect(m_process, &QProcess::errorOccurred, this, &PythonAddonRuntime::onProcessErrorOccurred);

    QString entryPath = m_pluginDir + "/" + m_manifest.entry;

    m_process->setWorkingDirectory(m_pluginDir);
    m_process->start(pythonPath(), {entryPath});

    if (!m_process->waitForStarted(5000)) {
        m_lastError = "Failed to start Python plugin process";
        delete m_process;
        m_process = nullptr;
        return false;
    }

    return true;
}

void PythonAddonRuntime::stopProcess()
{
    if (!m_process) return;

    m_process->kill();
    if (!m_process->waitForFinished(3000)) {
        m_process->kill();
        m_process->waitForFinished(1000);
    }
    delete m_process;
    m_process = nullptr;
}

void PythonAddonRuntime::sendMessage(const QJsonObject &msg)
{
    if (!m_process || m_process->state() != QProcess::Running)
        return;

    QByteArray data = QJsonDocument(msg).toJson(QJsonDocument::Compact) + "\n";
    m_process->write(data);
}

void PythonAddonRuntime::handleMessage(const QJsonObject &msg)
{
    if (msg.contains("method")) {
        QString method = msg["method"].toString();
        QJsonObject params = msg["params"].toObject();

        if (method == "log") {
            QString text = params["message"].toString();
            QString level = params["level"].toString("info");
            emit logMessage(text, level);
        } else if (method == "notify") {
            emit m_api->notify(params["title"].toString(), params["message"].toString());
        } else if (method == "register_command") {
            QString name = params["name"].toString();
            if (!name.isEmpty()) {
                auto *api = m_api;
                api->registerCommand(name, [this, name](const QJsonArray &args) -> QJsonValue {
                    return executeCommand(name, args);
                });
            }
        } else if (method == "create_file") {
            QString path = params["path"].toString();
            QString content = params["content"].toString();
            m_api->createFile(path, content.toUtf8());
        } else if (method == "read_file") {
            QString path = params["path"].toString();
            QByteArray data = m_api->readFileData(path);
            QJsonObject resp;
            resp["method"] = "read_file_result";
            resp["params"] = QJsonObject{{"path", path}, {"content", QString::fromUtf8(data)}};
            resp["id"] = msg.value("id").toInt();
            sendMessage(resp);
        } else if (method == "write_file") {
            QString path = params["path"].toString();
            QString content = params["content"].toString();
            m_api->writeFile(path, content.toUtf8());
        } else if (method == "delete_file") {
            QString path = params["path"].toString();
            m_api->deleteFileData(path);
        } else if (method == "list_files") {
            QString dir = params["dir"].toString();
            QStringList files = m_api->listFiles(dir);
            QJsonArray arr;
            for (const auto &f : files) arr.append(f);
            QJsonObject resp;
            resp["method"] = "list_files_result";
            resp["params"] = QJsonObject{{"files", arr}};
            resp["id"] = msg.value("id").toInt();
            sendMessage(resp);
        } else if (method == "create_menu") {
            QString parentPath = params["parent"].toString();
            QString label = params["label"].toString();
            QString commandId = params["command"].toString();
            m_api->createMenu(parentPath, label, commandId);
        }
    }
}

QString PythonAddonRuntime::venvPath() const
{
    return m_pluginDir + "/.venv";
}

QString PythonAddonRuntime::pythonPath() const
{
#ifdef Q_OS_WIN
    return venvPath() + "/Scripts/python.exe";
#else
    return venvPath() + "/bin/python3";
#endif
}
