#include <QApplication>
#include "background_download.h"
#include "mainwindow.h"

auto main(int argc, char *argv[]) -> int
{
    if (isBackgroundDownload(argc, argv))
        return runBackgroundDownload(argc, argv);

    QApplication app(argc, argv);
    app.setApplicationName("lazydesktop");
    app.setApplicationVersion("0.1.0");

    MainWindow window;
    window.show();

    return app.exec();
}
