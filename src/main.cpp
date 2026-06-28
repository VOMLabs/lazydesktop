#include <QApplication>
#include "mainwindow.h"

auto main(int argc, char *argv[]) -> int
{
    QApplication app(argc, argv);
    app.setApplicationName("lazydesktop");
    app.setApplicationVersion("0.1.0");

    MainWindow window;
    window.show();

    return app.exec();
}
