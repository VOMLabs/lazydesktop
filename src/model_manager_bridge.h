#ifndef MODEL_MANAGER_BRIDGE_H
#define MODEL_MANAGER_BRIDGE_H

#include <QObject>
#include <QString>
#include <QJsonArray>
#include <atomic>

struct ModelManager;

class ModelManagerBridge : public QObject
{
    Q_OBJECT

public:
    explicit ModelManagerBridge(const QString &modelsDir,
                                const QString &configPath,
                                QObject *parent = nullptr);
    ~ModelManagerBridge() override;

    int downloadModel(const QString &url, const QString &destPath,
                      const QString &expectedSha256 = {});
    void cancelDownload(int downloadId);
    QJsonArray listLocalModels();
    QJsonArray discoverModels();
    bool deleteModel(const QString &path);

    void streamInference(const QString &modelPath, const QString &prompt,
                         int nGpuLayers);
    void generateCommitMessage(const QString &modelPath,
                               const QJsonObject &context, int nGpuLayers);
    void cancelInference();
    bool isRunning() const;

signals:
    void downloadProgress(int downloadId, qint64 received, qint64 total);
    void downloadFinished(int downloadId, bool success, const QString &error);
    void inferenceFinished(const QString &text);
    void inferenceError(const QString &error);
    void inferenceToken(const QString &token);

private:
    static void onDownloadProgress(int id, int64_t received, int64_t total, void *ud);
    static void onInferenceToken(const char *token, void *ud);
    static void onInferenceError(const char *error, void *ud);
    static void onInferenceFinished(const char *message, void *ud);
    static bool onInferenceCancelled(void *ud);

    ModelManager *m_mm = nullptr;
    std::atomic<bool> m_inferenceCancelled{false};
    QString m_inferenceBuffer;
};

#endif // MODEL_MANAGER_BRIDGE_H
