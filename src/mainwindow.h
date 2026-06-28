#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QProcess>

#include <QMap>

class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QPlainTextEdit;
class QPushButton;
class QTextEdit;
class QTreeWidget;
class QTreeWidgetItem;

class MainWindow : public QMainWindow
{
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);
    ~MainWindow() override;

private slots:
    void onProjectButtonClicked();
    void onRecentProjectClicked(QListWidgetItem *item);
    void onTreeItemClicked(QTreeWidgetItem *item, int column);
    void onSummaryTextChanged(const QString &text);
    void onCommitClicked();
    void onCommitFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onCommitErrorOccurred(QProcess::ProcessError error);
    void onPushClicked();
    void onPushFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onPushErrorOccurred(QProcess::ProcessError error);
    void onGitProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onGitProcessErrorOccurred(QProcess::ProcessError error);

private:
    enum class GitQuery { None, Status, Unpushed };
    enum class PushState { Push, Fetch, Pull };

    void setupUi();
    bool openRepository(const QString &path);
    static bool isGitRepository(const QString &path);
    void startGitStatusQuery();
    void startGitUnpushedQuery();
    void addGitFileToTree(const QString &path, const QString &prefix);

    void loadRecentProjects();
    void saveRecentProjects();
    void addRecentProject(const QString &path);
    void populateRecentList();
    void setWorkspaceVisible(bool visible);

    QPushButton *m_projectButton = nullptr;
    QLabel *m_currentPathLabel = nullptr;
    QWidget *m_recentDrawer = nullptr;
    QPushButton *m_openProjectButton = nullptr;
    QListWidget *m_recentList = nullptr;
    QTreeWidget *m_gitStatusTree = nullptr;
    QPlainTextEdit *m_fileContentViewer = nullptr;

    QLineEdit *m_summaryInput = nullptr;
    QTextEdit *m_descriptionInput = nullptr;
    QPushButton *m_commitButton = nullptr;
    QProcess *m_commitProcess = nullptr;

    PushState m_pushState = PushState::Push;
    QPushButton *m_pushButton = nullptr;
    QProcess *m_pushProcess = nullptr;

    QProcess *m_gitProcess = nullptr;
    QString m_repoPath;
    GitQuery m_currentQuery = GitQuery::None;
    QMap<QString, QTreeWidgetItem *> m_treeDirs;
    QStringList m_recentProjects;
};

#endif // MAINWINDOW_H
