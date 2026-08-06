#include "background_download.h"

#include "ai_core.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QThread>
#include <atomic>
#include <cstring>

namespace {

struct BackgroundDownloadContext
{
    QString dest;
    std::atomic<int64_t> lastWriteMs{0};
    std::atomic<bool> done{false};
};

void writeStatusFile(const QString &dest, const QString &status, qint64 received,
                     qint64 total, const QString &error, qint64 pid)
{
    QJsonObject obj;
    obj["status"] = status;
    obj["received"] = received;
    obj["total"] = total;
    if (total > 0)
        obj["percent"] = static_cast<int>(100 * received / total);
    if (!error.isEmpty())
        obj["error"] = error;
    obj["pid"] = pid;
    obj["updated_at"] = QDateTime::currentSecsSinceEpoch();

    const QString path = bgStatusPath(dest);
    const QString tmp = path + QStringLiteral(".tmp");
    QFile file(tmp);
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        return;
    file.write(QJsonDocument(obj).toJson(QJsonDocument::Compact));
    file.close();
    QFile::remove(path);
    QFile::rename(tmp, path);
}

void onBgProgress(int, int64_t received, int64_t total, void *ud)
{
    auto *ctx = static_cast<BackgroundDownloadContext *>(ud);
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (now - ctx->lastWriteMs.load(std::memory_order_relaxed) < 250)
        return;
    ctx->lastWriteMs.store(now, std::memory_order_relaxed);
    writeStatusFile(ctx->dest, QStringLiteral("downloading"), received, total, {},
                    QCoreApplication::applicationPid());
}

void onBgFinished(int, int status, const char *error, void *ud)
{
    auto *ctx = static_cast<BackgroundDownloadContext *>(ud);
    const QString msg = error ? QString::fromUtf8(error) : QString();
    const QString state = status == 0   ? QStringLiteral("done")
                          : status == 5 ? QStringLiteral("cancelled")
                                        : QStringLiteral("error");
    writeStatusFile(ctx->dest, state, 0, 0, msg, QCoreApplication::applicationPid());
    ctx->done.store(true, std::memory_order_relaxed);
}

int backgroundDownloadImpl(const QString &url, const QString &dest,
                           const QString &expectedSha, const QString &modelsDir,
                           const QString &configPath)
{
    QDir().mkpath(QFileInfo(dest).absolutePath());

    ModelManager *mm = mm_init(modelsDir.toUtf8().constData(),
                               configPath.toUtf8().constData());
    if (!mm)
        return 1;

    BackgroundDownloadContext ctx;
    ctx.dest = dest;

    writeStatusFile(dest, QStringLiteral("downloading"), 0, 0, {},
                    QCoreApplication::applicationPid());

    const QByteArray urlUtf8 = url.toUtf8();
    const QByteArray destUtf8 = dest.toUtf8();
    const QByteArray shaUtf8 = expectedSha.toUtf8();
    const int id = mm_download_model(
        mm, urlUtf8.constData(), destUtf8.constData(),
        expectedSha.isEmpty() ? nullptr : shaUtf8.constData(),
        onBgProgress, onBgFinished, &ctx);
    if (id < 0) {
        mm_destroy(mm);
        return 1;
    }

    const QString cancelMarker = bgCancelPath(dest);
    while (!ctx.done.load(std::memory_order_relaxed)) {
        if (QFile::exists(cancelMarker))
            mm_cancel_download(mm, id);
        QThread::msleep(200);
    }

    mm_destroy(mm);
    QFile::remove(cancelMarker);
    return 0;
}

} // namespace

bool isBackgroundDownload(int argc, char *argv[])
{
    return argc >= 7 && std::strcmp(argv[1], "--background-dl") == 0;
}

int runBackgroundDownload(int argc, char *argv[])
{
    QCoreApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("lazydesktop"));
    return backgroundDownloadImpl(QString::fromLocal8Bit(argv[2]),
                                  QString::fromLocal8Bit(argv[3]),
                                  QString::fromLocal8Bit(argv[4]),
                                  QString::fromLocal8Bit(argv[5]),
                                  QString::fromLocal8Bit(argv[6]));
}

QString bgStatusPath(const QString &modelPath)
{
    return modelPath + QStringLiteral(".json");
}

QString bgCancelPath(const QString &modelPath)
{
    return modelPath + QStringLiteral(".cancel");
}

QString bgPartialPath(const QString &modelPath)
{
    return modelPath + QStringLiteral(".part");
}

bool bgProcessAlive(qint64 pid)
{
    if (pid <= 0)
        return false;
#ifdef Q_OS_LINUX
    return QFileInfo::exists(QStringLiteral("/proc/%1").arg(pid));
#else
    Q_UNUSED(pid);
    return true;
#endif
}

void writeBgStatus(const QString &dest, const QString &status, qint64 received,
                   qint64 total, const QString &error, qint64 pid)
{
    writeStatusFile(dest, status, received, total, error, pid);
}

bool spawnBackgroundDownload(const QString &url, const QString &dest,
                             const QString &modelsDir, const QString &configPath)
{
    const QString exe = QCoreApplication::applicationFilePath();
    const QStringList args{QStringLiteral("--background-dl"),
                           url, dest, QString(), modelsDir, configPath};
    qint64 pid = 0;
    if (!QProcess::startDetached(exe, args, QDir::currentPath(), &pid))
        return false;
    writeBgStatus(dest, QStringLiteral("downloading"), 0, 0, {}, pid);
    return true;
}
