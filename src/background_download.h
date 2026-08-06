#ifndef BACKGROUND_DOWNLOAD_H
#define BACKGROUND_DOWNLOAD_H

#include <QString>

/* Detached "model installer" mode. The main process relaunches itself with
   `--background-dl <url> <dest> <sha256> <modelsDir> <configPath>` and exits;
   this worker downloads <url> to <dest> (via a `.part` file) independently of
   the GUI, then writes a small JSON sidecar next to the model so the UI can
   show progress, detect completion, and request cancellation. */

bool isBackgroundDownload(int argc, char *argv[]);
int runBackgroundDownload(int argc, char *argv[]);

/* Sidecar paths for a model at `modelPath`. */
QString bgStatusPath(const QString &modelPath);
QString bgCancelPath(const QString &modelPath);
QString bgPartialPath(const QString &modelPath);

/* True when the background worker with `pid` is still running. */
bool bgProcessAlive(qint64 pid);

/* Atomically write the progress/status sidecar for a download. */
void writeBgStatus(const QString &dest, const QString &status, qint64 received,
                   qint64 total, const QString &error, qint64 pid);

/* Start the detached worker. On success also seeds the status sidecar so the
   UI immediately shows the download row. */
bool spawnBackgroundDownload(const QString &url, const QString &dest,
                             const QString &modelsDir, const QString &configPath);

#endif // BACKGROUND_DOWNLOAD_H
