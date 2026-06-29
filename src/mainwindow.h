#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QProcess>

#include <QFileSystemWatcher>
#include <QTimer>

class DiffViewer;
class QAction;
class QCheckBox;
class QComboBox;
class QDialog;
class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QMenu;
class QPlainTextEdit;
class QPushButton;
class QStackedWidget;
class QTabWidget;
class QTextEdit;
class QTreeWidget;
class QTreeWidgetItem;

struct GitCredentials
{
    QString host;
    QString username;
    QString token;
    bool valid = false;
};

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
    void onBranchChanged(int index);
    void onBranchesLoaded();
    void onCheckoutFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onCheckoutErrorOccurred(QProcess::ProcessError error);
    void onDeleteBranch();
    void onPushClicked();
    void onPushFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onPushErrorOccurred(QProcess::ProcessError error);
    void onInstallFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onAuthCheckFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onGitProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onGitProcessErrorOccurred(QProcess::ProcessError error);
    void onLogFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onHistoryItemClicked(QListWidgetItem *item);
    void onCommitFileClicked(QListWidgetItem *item);
    void onCommitDetailFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onRepoDirChanged();
    void onRefreshDebounce();
    void onOpenEditor();
    void onOpenFileManager();
    void onOpenTerminal();
    void onOpenGitHub();
    void closeRepository();
    void onTreeContextMenu(const QPoint &pos);
    void onDiscardFile();
    void toggleCommitPanel(bool visible);
    void toggleCommitFilesPanel(bool visible);

private:
    enum class GitQuery { None, Status, Unpushed };
    enum class PushState { Push, Fetch, Pull };

    void setupUi();
    bool openRepository(const QString &path);
    static bool isGitRepository(const QString &path);
    void startGitStatusQuery();
    void startGitUnpushedQuery();
    void startGitLogQuery();
    void addGitFileToTree(const QString &path, const QString &prefix);
    void setAllCheckStates(Qt::CheckState state);
    QStringList checkedFiles() const;

    void loadBranches();
    void refreshAll();
    void doCheckout(const QString &branch);
    void restoreBranchSelection();
    void updateDeleteButtonState();
    void createNewBranch();

    void loadRecentProjects();
    void saveRecentProjects();
    void addRecentProject(const QString &path);
    void populateRecentList();
    void onRemoveRecentProject();

    bool checkGitAvailable();
    void installGit();

    void checkGitAuth();
    void showAuthDialog();
    bool setupAskPass();
    void cleanupAskPass();
    void setupAuthEnv(QProcess *proc);

    QCheckBox *m_selectAllCheck = nullptr;
    QLabel *m_changedFilesLabel = nullptr;
    QWidget *m_headerBar = nullptr;
    QProcess *m_stageProcess = nullptr;
    QPushButton *m_projectButton = nullptr;
    QLabel *m_currentPathLabel = nullptr;
    QWidget *m_recentDrawer = nullptr;
    QPushButton *m_openProjectButton = nullptr;
    QListWidget *m_recentList = nullptr;
    QTreeWidget *m_gitStatusTree = nullptr;
    DiffViewer *m_fileContentViewer = nullptr;
    QAction *m_openGitHubAction = nullptr;
    QAction *m_viewCommitPanelAction = nullptr;
    QAction *m_viewCommitFilesAction = nullptr;
    QWidget *m_commitContainer = nullptr;
    QWidget *m_placeholderWidget = nullptr;
    QStackedWidget *m_viewerStack = nullptr;
    QTabWidget *m_sidebarTabs = nullptr;
    QListWidget *m_commitHistoryList = nullptr;
    QListWidget *m_commitFilesList = nullptr;
    QWidget *m_commitFilesHeader = nullptr;

    QLineEdit *m_summaryInput = nullptr;
    QTextEdit *m_descriptionInput = nullptr;
    QPushButton *m_commitButton = nullptr;
    QProcess *m_commitProcess = nullptr;

    PushState m_pushState = PushState::Push;
    QPushButton *m_pushButton = nullptr;
    QProcess *m_pushProcess = nullptr;

    QComboBox *m_branchComboBox = nullptr;
    QPushButton *m_deleteBranchButton = nullptr;
    QProcess *m_branchProcess = nullptr;
    QProcess *m_checkoutProcess = nullptr;
    QProcess *m_createBranchProcess = nullptr;
    QProcess *m_logProcess = nullptr;
    QProcess *m_commitDetailProcess = nullptr;
    QString m_selectedCommitHash;
    QString m_currentBranch;
    bool m_populatingBranches = false;

    QProcess *m_installProcess = nullptr;
    QProcess *m_authProcess = nullptr;
    GitCredentials m_gitCredentials;
    QString m_askPassScriptPath;

    QFileSystemWatcher *m_fsWatcher = nullptr;
    QTimer *m_refreshTimer = nullptr;
    QProcess *m_gitProcess = nullptr;
    QString m_repoPath;
    GitQuery m_currentQuery = GitQuery::None;
    QStringList m_recentProjects;
};

#endif // MAINWINDOW_H
