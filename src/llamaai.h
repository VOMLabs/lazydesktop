#ifndef LLAMAAI_H
#define LLAMAAI_H

#include <QObject>
#include <QString>
#include <QThread>
#include <atomic>
#include <memory>
#include <string>
#include <vector>

struct llama_model;
struct llama_context;
struct llama_sampler;

class LlamaWorker : public QObject
{
    Q_OBJECT

public:
    explicit LlamaWorker(QObject *parent = nullptr);
    ~LlamaWorker() override;

    void setModelPath(const QString &path);
    void setPrompt(const QString &systemPrompt, const QString &userContent);
    void setGpuAcceleration(bool enabled);
    void cancel();

public slots:
    void run();

signals:
    void finished(const QString &text);
    void errorOccurred(const QString &error);
    void thinking(const QString &token);

private:
    bool loadModel();
    void unloadModel();
    std::string formatPrompt() const;

    QString m_modelPath;
    QString m_systemPrompt;
    QString m_userContent;
    bool m_gpuAccel = true;
    std::atomic<bool> m_cancelled{false};

    llama_model *m_model = nullptr;
    llama_context *m_ctx = nullptr;
    llama_sampler *m_smpl = nullptr;
};

class LlamaAI : public QObject
{
    Q_OBJECT

public:
    explicit LlamaAI(QObject *parent = nullptr);
    ~LlamaAI() override;

    void setModelPath(const QString &path);
    QString modelPath() const;
    void setGpuAcceleration(bool enabled);

    void generate(const QString &systemPrompt, const QString &userContent);
    void cancel();
    bool isRunning() const;

signals:
    void finished(const QString &text);
    void errorOccurred(const QString &error);
    void thinking(const QString &token);

private:
    QString m_modelPath;
    bool m_gpuAccel = true;
    QThread *m_thread = nullptr;
    LlamaWorker *m_worker = nullptr;
};

#endif // LLAMAAI_H