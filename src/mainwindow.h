#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QMainWindow>
#include <QProcess>

#include <QFileSystemWatcher>
#include <QNetworkAccessManager>
#include <QTimer>

class DiffViewer;
class QAction;
class QCheckBox;
class QComboBox;
class QDialog;
class QJsonObject;
class QLabel;
class QLineEdit;
class QListWidget;
class QListWidgetItem;
class QMenu;
class QNetworkReply;
class QPlainTextEdit;
class QPushButton;
class QStackedWidget;
class QTabWidget;
class QTextEdit;
class QTreeWidget;
class QTreeWidgetItem;

class ModelManagerBridge;
class VcsBridge;
class AddonBridge;

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
    explicit MainWindow(QWidget* parent = nullptr);
    ~MainWindow() override;

protected:
    void closeEvent(QCloseEvent* event) override;

private slots:
    void onProjectButtonClicked();
    void onRecentProjectClicked(QListWidgetItem* item);
    void onTreeItemClicked(QTreeWidgetItem* item, int column);
    void onSummaryTextChanged(const QString& text);
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
    void manageRemotes();
    void onInstallFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onAuthCheckFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onGitProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onGitProcessErrorOccurred(QProcess::ProcessError error);
    void onLogFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onHistoryItemClicked(QListWidgetItem* item);
    void onCommitFileClicked(QListWidgetItem* item);
    void onCommitDetailFinished(int exitCode, QProcess::ExitStatus exitStatus);
    void onRepoDirChanged();
    void onRefreshDebounce();
    void onOpenEditor();
    void onOpenFileManager();
    void onOpenTerminal();
    void onOpenGitHub();
    void onRepositorySettings();
    void onOpenSettings();
    void onGenerateCommitMessage();
    void onGenerateCommitDescription();
    void onEnableAiSystem();
    void onLocalAiResponse(const QString& text);
    void onLocalAiError(const QString& error);
    void onLocalAiThinking(const QString& token);
    void readDiffForFile(const QString& file) const;
    void onAddCoAuthors();
    void onStageAllFiles();
    void onUnstageAllFiles();
    void closeRepository();
    void onTreeContextMenu(const QPoint& pos);
    void onDiscardFile();
    void toggleCommitPanel(bool visible);
    void toggleCommitFilesPanel(bool visible);


private:
    enum class GitQuery
    {
        None,
        Status,
        Unpushed
    };
    enum class PushState
    {
        Push,
        Fetch,
        Pull
    };
    enum class AiRequestKind
    {
        CommitMessage,
        Description
    };

    void runAiGeneration(AiRequestKind kind, const QString& rawPrompt);

    void setupUi();
    bool openRepository(const QString& path);
    static bool isGitRepository(const QString& path);
    static bool isDirtyRepository(const QString& path);
    bool isJjRepo() const;
    void startGitStatusQuery();
    void startGitUnpushedQuery();
    void startGitLogQuery();
    void commitJj();
    void addGitFileToTree(const QString& path, const QString& prefix, QTreeWidgetItem* parent = nullptr);
    void setAllCheckStates(Qt::CheckState state);
    QStringList checkedFiles() const;
    void updateStagedUnstagedTrees(const QString& output);

    void loadBranches();
    void refreshAll();
    void doCheckout(const QString& branch);
    void restoreBranchSelection();
    void updateDeleteButtonState();
    void createNewBranch();

    void loadRecentProjects();
    void saveRecentProjects();
    void addRecentProject(const QString& path);
    void populateRecentList();
    void onRemoveRecentProject();
    void onClearAllProjects();
    void onScanFolder();
    void onCloneRepository();
    void onCreateRepository();
    void onOpenExistingProject();
    void applySavedTheme();
    void onAiResponse(QNetworkReply* reply);
    void showRepositorySettingsDialog(const QJsonObject& repo);

    bool checkGitAvailable();
    void installGit();

    void checkGitAuth();
    void showAuthDialog();
    bool setupAskPass();
    void cleanupAskPass();
    void setupAuthEnv(QProcess* proc);

    QCheckBox* m_selectAllCheck = nullptr;
    QLabel* m_changedFilesLabel = nullptr;
    QWidget* m_headerBar = nullptr;
    QProcess* m_stageProcess = nullptr;
    QPushButton* m_projectButton = nullptr;
    QLabel* m_currentPathLabel = nullptr;
    QWidget* m_recentDrawer = nullptr;
    QPushButton* m_addProjectButton = nullptr;
    QListWidget* m_recentList = nullptr;
    QTreeWidget* m_gitStatusTree = nullptr;
    QAction* m_openGitHubAction = nullptr;
    QAction* m_repoSettingsAction = nullptr;
    QAction* m_viewCommitPanelAction = nullptr;
    QAction* m_viewCommitFilesAction = nullptr;
    DiffViewer* m_fileContentViewer = nullptr;
    QLabel* m_binaryPreview = nullptr;
    QWidget* m_placeholderWidget = nullptr;
    QStackedWidget* m_viewerStack = nullptr;
    QWidget* m_commitFilesHeader = nullptr;
    QWidget* m_commitFilesContainer = nullptr;
    QWidget* m_commitContainer = nullptr;
    QTabWidget* m_sidebarTabs = nullptr;
    QListWidget* m_commitHistoryList = nullptr;
    QListWidget* m_commitFilesList = nullptr;

    QLineEdit* m_summaryInput = nullptr;
    QTextEdit* m_descriptionInput = nullptr;
    QPushButton* m_commitButton = nullptr;
    QPushButton* m_aiCommitButton = nullptr;
    QPushButton* m_aiDescriptionButton = nullptr;
    AiRequestKind m_aiRequestKind = AiRequestKind::CommitMessage;
    QPushButton* m_skipHooksButton = nullptr;
    QPushButton* m_coAuthorButton = nullptr;
    QPushButton* m_aiEnableButton = nullptr;
    QLabel* m_aiExperimentalLabel = nullptr;
    QWidget* m_aiRowContainer = nullptr;
    QProcess* m_commitProcess = nullptr;
    QNetworkAccessManager* m_networkManager = nullptr;

    // AI thinking indicator
    QWidget* m_aiThinkingOverlay = nullptr;
    QLabel* m_aiThinkingLabel = nullptr;
    QPushButton* m_aiShowMoreButton = nullptr;
    QTextEdit* m_aiThinkingText = nullptr;
    bool m_aiThinkingVisible = false;

    PushState m_pushState = PushState::Push;
    bool m_pullRebasePending = false;
    QPushButton* m_pushButton = nullptr;
    QProcess* m_pushProcess = nullptr;
    QPushButton* m_remotesButton = nullptr;

    QComboBox* m_branchComboBox = nullptr;
    QPushButton* m_deleteBranchButton = nullptr;
    QProcess* m_branchProcess = nullptr;
    QProcess* m_checkoutProcess = nullptr;
    QProcess* m_createBranchProcess = nullptr;
    QProcess* m_logProcess = nullptr;
    QProcess* m_commitDetailProcess = nullptr;
    QString m_selectedCommitHash;
    QString m_currentBranch;
    bool m_populatingBranches = false;

    QProcess* m_installProcess = nullptr;
    QProcess* m_authProcess = nullptr;
    GitCredentials m_gitCredentials;
    QString m_askPassScriptPath;

    QFileSystemWatcher* m_fsWatcher = nullptr;
    QTimer* m_refreshTimer = nullptr;
    QProcess* m_gitProcess = nullptr;
    QString m_repoPath;
    GitQuery m_currentQuery = GitQuery::None;
    QStringList m_recentProjects;
    ModelManagerBridge* m_modelManager = nullptr;
    VcsBridge* m_vcsBridge = nullptr;
    AddonBridge* m_addonBridge = nullptr;
    bool m_aiEnabled = false;
};

#endif // MAINWINDOW_H
