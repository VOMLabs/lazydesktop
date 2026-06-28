#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QProcess>

#include <QMap>

class QLabel;
class QPushButton;
class QTreeWidget;
class QTreeWidgetItem;

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override;

private slots:
    void onOpenFolder();
    void onGitProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onGitProcessErrorOccurred(QProcess::ProcessError error);

private:
    enum class GitQuery { None, Status, Unpushed };

    void setupUi();
    static bool isGitRepository(const QString &path);
    void startGitStatusQuery();
    void startGitUnpushedQuery();
    void addGitFileToTree(const QString &path, const QString &prefix);

    QPushButton *m_openFolderButton = nullptr;
    QLabel *m_currentPathLabel = nullptr;
    QTreeWidget *m_gitStatusTree = nullptr;
    QProcess *m_gitProcess = nullptr;
    QString m_repoPath;
    GitQuery m_currentQuery = GitQuery::None;
    QMap<QString, QTreeWidgetItem *> m_treeDirs;
};

#endif // MAINWINDOW_H
