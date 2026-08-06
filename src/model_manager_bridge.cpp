#include "model_manager_bridge.h"
#include "model_manager.h"

#include <QDebug>
#include <QJsonDocument>
#include <QJsonObject>

// ─── Static callbacks ──────────────────────────────────

void ModelManagerBridge::onDownloadProgress(int id, int64_t received, int64_t total, void *ud)
{
    auto *self = static_cast<ModelManagerBridge *>(ud);
    emit self->downloadProgress(id, static_cast<qint64>(received),
                                static_cast<qint64>(total));
}

void ModelManagerBridge::onInferenceToken(const char *token, void *ud)
{
    auto *self = static_cast<ModelManagerBridge *>(ud);
    const QString text = QString::fromUtf8(token);
    self->m_inferenceBuffer += text;
    emit self->inferenceToken(text);
}

void ModelManagerBridge::onInferenceError(const char *error, void *ud)
{
    auto *self = static_cast<ModelManagerBridge *>(ud);
    const QString msg = QString::fromUtf8(error);
    emit self->inferenceError(msg);
}

void ModelManagerBridge::onInferenceFinished(const char *message, void *ud)
{
    auto *self = static_cast<ModelManagerBridge *>(ud);
    const QString msg = QString::fromUtf8(message);
    if (!msg.isEmpty())
        self->m_inferenceBuffer = msg;
    emit self->inferenceFinished(msg);
}

bool ModelManagerBridge::onInferenceCancelled(void *ud)
{
    auto *self = static_cast<ModelManagerBridge *>(ud);
    return self->m_inferenceCancelled.load();
}

// ─── Lifetime ──────────────────────────────────────────

ModelManagerBridge::ModelManagerBridge(const QString &modelsDir,
                                       const QString &configPath,
                                       QObject *parent)
    : QObject(parent)
{
    m_mm = mm_init(modelsDir.toUtf8().constData(),
                   configPath.toUtf8().constData());
    if (!m_mm)
        qWarning() << "ModelManagerBridge: mm_init failed";
}

ModelManagerBridge::~ModelManagerBridge()
{
    cancelInference();
    if (m_mm) {
        mm_destroy(m_mm);
        m_mm = nullptr;
    }
}

// ─── Download ──────────────────────────────────────────

int ModelManagerBridge::downloadModel(const QString &url, const QString &destPath,
                                      const QString &expectedSha256)
{
    if (!m_mm)
        return -1;

    const QByteArray urlUtf8 = url.toUtf8();
    const QByteArray destUtf8 = destPath.toUtf8();
    const QByteArray shaUtf8 = expectedSha256.toUtf8();

    return mm_download_model(
        m_mm,
        urlUtf8.constData(),
        destUtf8.constData(),
        expectedSha256.isEmpty() ? nullptr : shaUtf8.constData(),
        &ModelManagerBridge::onDownloadProgress,
        this);
}

void ModelManagerBridge::cancelDownload(int downloadId)
{
    if (m_mm)
        mm_cancel_download(m_mm, downloadId);
}

// ─── Model listing ─────────────────────────────────────

QJsonArray ModelManagerBridge::listLocalModels()
{
    if (!m_mm)
        return {};

    char *json = mm_list_local_models(m_mm);
    if (!json)
        return {};

    QJsonDocument doc = QJsonDocument::fromJson(QByteArray(json));
    mm_free_string(json);

    if (!doc.isArray())
        return {};

    return doc.array();
}

QJsonArray ModelManagerBridge::discoverModels()
{
    if (!m_mm)
        return {};

    char *json = mm_discover_models(m_mm);
    if (!json)
        return {};

    QJsonDocument doc = QJsonDocument::fromJson(QByteArray(json));
    mm_free_string(json);

    if (!doc.isArray())
        return {};

    return doc.array();
}

// ─── Delete ────────────────────────────────────────────

bool ModelManagerBridge::deleteModel(const QString &path)
{
    if (!m_mm)
        return false;

    const QByteArray pathUtf8 = path.toUtf8();
    return mm_delete_model(m_mm, pathUtf8.constData());
}

// ─── Inference ─────────────────────────────────────────

void ModelManagerBridge::streamInference(const QString &modelPath,
                                         const QString &prompt,
                                         int nGpuLayers)
{
    if (!m_mm)
        return;

    m_inferenceCancelled = false;
    m_inferenceBuffer.clear();

    const QByteArray pathUtf8 = modelPath.toUtf8();
    const QByteArray promptUtf8 = prompt.toUtf8();

    mm_stream_inference(
        m_mm,
        pathUtf8.constData(),
        promptUtf8.constData(),
        nGpuLayers,
        &ModelManagerBridge::onInferenceToken,
        &ModelManagerBridge::onInferenceError,
        &ModelManagerBridge::onInferenceCancelled,
        &ModelManagerBridge::onInferenceFinished,
        this);
}

void ModelManagerBridge::generateCommitMessage(const QString &modelPath,
                                               const QJsonObject &context,
                                               int nGpuLayers)
{
    if (!m_mm)
        return;

    m_inferenceCancelled = false;
    m_inferenceBuffer.clear();

    const QByteArray pathUtf8 = modelPath.toUtf8();
    const QByteArray contextUtf8 = QJsonDocument(context).toJson(QJsonDocument::Compact);

    mm_generate_commit_message(
        m_mm,
        pathUtf8.constData(),
        contextUtf8.constData(),
        nGpuLayers,
        &ModelManagerBridge::onInferenceToken,
        &ModelManagerBridge::onInferenceError,
        &ModelManagerBridge::onInferenceCancelled,
        &ModelManagerBridge::onInferenceFinished,
        this);
}

void ModelManagerBridge::cancelInference()
{
    m_inferenceCancelled = true;
}

bool ModelManagerBridge::isRunning() const
{
    return false; // Inference runs on its own thread; bridge doesn't track thread state
}
