#include "diffviewer.h"
#include "mainwindow.h"

#include <QCheckBox>
#include <QComboBox>
#include <QDesktopServices>
#include <QDialog>
#include <QDialogButtonBox>
#include <QDirIterator>
#include <QDir>
#include <QFile>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFont>
#include <QFontDatabase>
#include <QFormLayout>
#include <QGraphicsDropShadowEffect>
#include <QHBoxLayout>
#include <QInputDialog>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcess>
#include <QProcessEnvironment>
#include <QPushButton>
#include <QRegularExpression>
#include <QStyledItemDelegate>
#include <QPainter>
#include <QPixmap>
#include <QSettings>
#include <QSplitter>
#include <QStackedWidget>
#include <QStandardPaths>
#include <QTemporaryFile>
#include <QTextStream>
#include <QTextCursor>
#include <QTextEdit>
#include <QTreeWidget>
#include <QTreeWidgetItem>
#include <QTreeWidgetItemIterator>
#include <QUrl>
#include <QVBoxLayout>
#include <QStandardPaths>

#include <yaml-cpp/yaml.h>

class CommitDelegate : public QStyledItemDelegate {
public:
    using QStyledItemDelegate::QStyledItemDelegate;

    void paint(QPainter *painter, const QStyleOptionViewItem &option,
               const QModelIndex &index) const override
    {
        QStyleOptionViewItem opt = option;
        initStyleOption(&opt, index);

        painter->save();

        const bool selected = opt.state & QStyle::State_Selected;
        const bool hovered = opt.state & QStyle::State_MouseOver;

        if (selected) {
            painter->fillRect(opt.rect, opt.palette.highlight());
        } else if (hovered) {
            painter->fillRect(opt.rect, opt.palette.alternateBase());
        }

        const QString text = index.data(Qt::DisplayRole).toString();
        const QStringList parts = text.split('\n');
        const QString hash    = parts.value(0);
        const QString subject = parts.value(1);
        const QString meta    = parts.value(2);

        QRect r = opt.rect.adjusted(4, 2, -4, -2);
        const int lh = opt.fontMetrics.height();

        // Hash — monospace, small, muted
        {
            QFont f = opt.font;
            f.setFamilies({"monospace", "Courier New", "Liberation Mono", "Menlo", "Consolas"});
            f.setPointSize(f.pointSize() - 2);
            painter->setFont(f);
            painter->setPen(QColor("#888888"));
            painter->drawText(r.left(), r.top(), r.width(), lh,
                              Qt::AlignLeft | Qt::AlignBottom | Qt::TextSingleLine, hash);
        }

        // Subject — bold
        {
            QFont f = opt.font;
            f.setBold(true);
            painter->setFont(f);
            painter->setPen(selected ? opt.palette.highlightedText().color()
                                     : opt.palette.windowText().color());
            painter->drawText(r.left(), r.top() + lh, r.width(), lh,
                              Qt::AlignLeft | Qt::AlignVCenter | Qt::TextSingleLine, subject);
        }

        // Meta — smaller, gray
        if (!meta.isEmpty()) {
            QFont f = opt.font;
            f.setPointSize(f.pointSize() - 1);
            painter->setFont(f);
            painter->setPen(selected ? opt.palette.highlightedText().color().lighter(160)
                                     : opt.palette.color(QPalette::Disabled, QPalette::WindowText));
            painter->drawText(r.left(), r.top() + lh * 2, r.width(), lh,
                              Qt::AlignLeft | Qt::AlignTop | Qt::TextSingleLine, meta);
        }

        painter->restore();
    }

    QSize sizeHint(const QStyleOptionViewItem &option,
                   const QModelIndex &) const override
    {
        return QSize(200, option.fontMetrics.height() * 3 + 4);
    }
};

static const int kMaxRecentProjects = 100;

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setupUi();
    loadRecentProjects();
    checkGitAvailable();
}

MainWindow::~MainWindow()
{
    if (m_gitProcess && m_gitProcess->state() != QProcess::NotRunning) {
        m_gitProcess->kill();
        m_gitProcess->waitForFinished(3000);
    }
    if (m_commitProcess && m_commitProcess->state() != QProcess::NotRunning) {
        m_commitProcess->kill();
        m_commitProcess->waitForFinished(3000);
    }
    if (m_pushProcess && m_pushProcess->state() != QProcess::NotRunning) {
        m_pushProcess->kill();
        m_pushProcess->waitForFinished(3000);
    }
    if (m_branchProcess && m_branchProcess->state() != QProcess::NotRunning) {
        m_branchProcess->kill();
        m_branchProcess->waitForFinished(3000);
    }
    if (m_checkoutProcess && m_checkoutProcess->state() != QProcess::NotRunning) {
        m_checkoutProcess->kill();
        m_checkoutProcess->waitForFinished(3000);
    }
    if (m_installProcess && m_installProcess->state() != QProcess::NotRunning) {
        m_installProcess->kill();
        m_installProcess->waitForFinished(3000);
    }
    if (m_authProcess && m_authProcess->state() != QProcess::NotRunning) {
        m_authProcess->kill();
        m_authProcess->waitForFinished(3000);
    }
    if (m_createBranchProcess && m_createBranchProcess->state() != QProcess::NotRunning) {
        m_createBranchProcess->kill();
        m_createBranchProcess->waitForFinished(3000);
    }
    cleanupAskPass();
}

void MainWindow::closeEvent(QCloseEvent *event)
{
    saveRecentProjects();
    QMainWindow::closeEvent(event);
}

void MainWindow::setupUi()
{
    setWindowTitle("lazydesktop");
    resize(1000, 600);

    auto *centralWidget = new QWidget(this);
    auto *mainLayout = new QVBoxLayout(centralWidget);
    mainLayout->setContentsMargins(0, 0, 0, 0);
    mainLayout->setSpacing(0);

    // --- Top toolbar ---
    auto *topLayout = new QHBoxLayout();
    topLayout->setContentsMargins(6, 4, 6, 4);

    m_projectButton = new QPushButton("Open Folder");
    m_projectButton->setMinimumHeight(28);

    m_currentPathLabel = new QLabel();
    m_currentPathLabel->setVisible(false);

    m_pushButton = new QPushButton("Push");
    m_pushButton->setEnabled(false);
    m_pushState = PushState::Push;

    m_branchComboBox = new QComboBox();
    m_branchComboBox->setMinimumWidth(160);
    m_branchComboBox->setSizePolicy(QSizePolicy::Preferred, QSizePolicy::Fixed);
    m_branchComboBox->setEnabled(false);

    m_deleteBranchButton = new QPushButton(QString::fromUtf8("\u2716"));
    m_deleteBranchButton->setFixedSize(24, 24);
    m_deleteBranchButton->setEnabled(false);
    m_deleteBranchButton->setStyleSheet(
        "QPushButton { color: red; border: none; font-weight: bold; }"
        "QPushButton:hover { background: #ffcccc; }");

    topLayout->addWidget(m_projectButton);
    topLayout->addWidget(m_currentPathLabel);
    topLayout->addWidget(m_branchComboBox);
    topLayout->addWidget(m_deleteBranchButton);
    topLayout->addStretch();
    topLayout->addWidget(m_pushButton);

    // --- Files menu (in menubar) ---
    auto *menuBar = this->menuBar();
    auto *filesMenu = menuBar->addMenu("Files");

    auto *editorAction = filesMenu->addAction("Open in Editor");
    connect(editorAction, &QAction::triggered, this, &MainWindow::onOpenEditor);

    auto *fmAction = filesMenu->addAction("Open in File Manager");
    connect(fmAction, &QAction::triggered, this, &MainWindow::onOpenFileManager);

    auto *termAction = filesMenu->addAction("Open in Terminal");
    connect(termAction, &QAction::triggered, this, &MainWindow::onOpenTerminal);

    m_openGitHubAction = filesMenu->addAction("View on GitHub");
    m_openGitHubAction->setEnabled(false);
    connect(m_openGitHubAction, &QAction::triggered, this, &MainWindow::onOpenGitHub);

    filesMenu->addSeparator();

    auto *settingsAction = filesMenu->addAction("Settings...");
    connect(settingsAction, &QAction::triggered, this, &MainWindow::onOpenSettings);

    // --- View menu ---
    auto *viewMenu = menuBar->addMenu("View");

    m_viewCommitPanelAction = viewMenu->addAction("Commit Panel");
    m_viewCommitPanelAction->setCheckable(true);
    m_viewCommitPanelAction->setChecked(true);
    connect(m_viewCommitPanelAction, &QAction::toggled, this, &MainWindow::toggleCommitPanel);

    m_viewCommitFilesAction = viewMenu->addAction("Commit Files");
    m_viewCommitFilesAction->setCheckable(true);
    m_viewCommitFilesAction->setChecked(false);
    connect(m_viewCommitFilesAction, &QAction::toggled, this, &MainWindow::toggleCommitFilesPanel);

    // --- Recent projects drawer (overlay, hidden by default) ---
    m_recentDrawer = new QWidget(centralWidget);
    m_recentDrawer->setFixedWidth(260);
    m_recentDrawer->setVisible(false);
    m_recentDrawer->setAutoFillBackground(true);

    auto *drawerFrame = new QFrame(m_recentDrawer);
    drawerFrame->setFrameShape(QFrame::StyledPanel);
    drawerFrame->setFrameShadow(QFrame::Raised);

    auto *drawerLayout = new QVBoxLayout(m_recentDrawer);
    drawerLayout->setContentsMargins(0, 0, 0, 0);
    drawerLayout->addWidget(drawerFrame, 1);

    auto *frameLayout = new QVBoxLayout(drawerFrame);
    frameLayout->setContentsMargins(6, 6, 6, 6);

    m_openProjectButton = new QPushButton("+ Open Project");
    m_openProjectButton->setMinimumHeight(32);

    m_recentList = new QListWidget();
    m_recentList->setAlternatingRowColors(true);

    frameLayout->addWidget(m_openProjectButton);

    auto *scanButton = new QPushButton("Scan Folder for Projects");
    scanButton->setMinimumHeight(28);
    scanButton->setToolTip("Scan a folder for subdirectories that are Git repositories and add them all");
    connect(scanButton, &QPushButton::clicked, this, &MainWindow::onScanFolder);
    frameLayout->addWidget(scanButton);

    frameLayout->addWidget(m_recentList, 1);

    // --- Main content widgets ---
    m_gitStatusTree = new QTreeWidget();
    m_gitStatusTree->setHeaderHidden(true);
    m_gitStatusTree->setColumnCount(1);
    m_gitStatusTree->setContextMenuPolicy(Qt::CustomContextMenu);
    m_gitStatusTree->setRootIsDecorated(false);
    m_gitStatusTree->setIndentation(0);
    m_gitStatusTree->setAnimated(false);
    m_gitStatusTree->setIconSize(QSize(10, 10));
    m_gitStatusTree->setStyleSheet(
        "QTreeWidget::item:selected {"
        "  background: palette(highlight);"
        "  color: palette(highlighted-text);"
        "}"
        "QTreeWidget::item:selected:!active {"
        "  background: palette(highlight);"
        "  color: palette(highlighted-text);"
        "}"
    );

    // Header bar: master checkbox + changed-files count
    m_headerBar = new QWidget();
    m_headerBar->setFixedHeight(32);
    m_headerBar->setAutoFillBackground(true);
    {
        QPalette hp = m_headerBar->palette();
        hp.setColor(QPalette::Window, hp.color(QPalette::Window).darker(108));
        m_headerBar->setPalette(hp);
    }
    auto *headerLayout = new QHBoxLayout(m_headerBar);
    headerLayout->setContentsMargins(8, 0, 8, 0);
    m_selectAllCheck = new QCheckBox();
    m_changedFilesLabel = new QLabel("0 changed files");
    {
        QFont bf = m_changedFilesLabel->font();
        bf.setBold(true);
        m_changedFilesLabel->setFont(bf);
        QPalette lp = m_changedFilesLabel->palette();
        lp.setColor(QPalette::WindowText,
                    lp.color(QPalette::Disabled, QPalette::WindowText));
        m_changedFilesLabel->setPalette(lp);
    }
    headerLayout->addWidget(m_selectAllCheck);
    headerLayout->addWidget(m_changedFilesLabel, 1);

    m_fileContentViewer = new DiffViewer();
    m_fileContentViewer->setMinimumWidth(200);

    // Placeholder page shown when nothing is selected
    m_placeholderWidget = new QWidget();
    auto *placeholderLayout = new QVBoxLayout(m_placeholderWidget);
    placeholderLayout->setAlignment(Qt::AlignCenter);
    auto *placeholderLabel = new QLabel("No file selected");
    placeholderLabel->setAlignment(Qt::AlignCenter);
    placeholderLabel->setStyleSheet("color: gray; font-size: 14px;");
    placeholderLayout->addWidget(placeholderLabel);

    m_viewerStack = new QStackedWidget();
    m_viewerStack->addWidget(m_fileContentViewer);  // page 0
    m_viewerStack->addWidget(m_placeholderWidget);  // page 1
    m_viewerStack->setCurrentIndex(1);

    // --- Commit pane ---
    m_summaryInput = new QLineEdit();
    m_summaryInput->setPlaceholderText("Summary (required)");

    m_descriptionInput = new QTextEdit();
    m_descriptionInput->setPlaceholderText("Description (optional)");
    m_descriptionInput->setFixedHeight(80);
    m_descriptionInput->setAcceptRichText(false);

    m_commitButton = new QPushButton("Commit");
    m_commitButton->setEnabled(false);

    // Commit pane close button (top right)
    auto *commitHeader = new QWidget();
    auto *commitHeaderLayout = new QHBoxLayout(commitHeader);
    commitHeaderLayout->setContentsMargins(4, 2, 4, 2);
    auto *commitTitle = new QLabel("Commit");
    commitTitle->setStyleSheet("font-weight: bold; font-size: 12px;");
    auto *commitCloseBtn = new QPushButton(QStringLiteral("\u2716"));
    commitCloseBtn->setFixedSize(20, 20);
    commitCloseBtn->setFlat(true);
    commitCloseBtn->setStyleSheet("QPushButton { border: none; color: #8b949e; }"
                                   "QPushButton:hover { color: #e6edf3; }");
    commitHeaderLayout->addWidget(commitTitle, 1);
    commitHeaderLayout->addWidget(commitCloseBtn, 0, Qt::AlignRight);

    auto *commitLayout = new QVBoxLayout();
    commitLayout->setContentsMargins(0, 0, 0, 0);
    commitLayout->setSpacing(0);
    commitLayout->addWidget(commitHeader);
    auto *commitBody = new QWidget();
    auto *commitBodyLayout = new QVBoxLayout(commitBody);
    commitBodyLayout->setContentsMargins(4, 4, 4, 4);
    commitBodyLayout->addWidget(m_summaryInput);
    commitBodyLayout->addWidget(m_descriptionInput);
    commitBodyLayout->addWidget(m_commitButton);
    commitLayout->addWidget(commitBody, 1);

    auto *commitContainer = new QWidget();
    commitContainer->setLayout(commitLayout);
    m_commitContainer = commitContainer;

    connect(commitCloseBtn, &QPushButton::clicked, this, [this]() {
        m_commitContainer->setVisible(false);
        if (m_viewCommitPanelAction)
            m_viewCommitPanelAction->setChecked(false);
    });

    // Sidebar with tabs: Changes + History
    m_sidebarTabs = new QTabWidget();

    // Tab 1 — Changes
    auto *changesTab = new QWidget();
    auto *changesLayout = new QVBoxLayout(changesTab);
    changesLayout->setContentsMargins(0, 0, 0, 0);
    changesLayout->setSpacing(0);
    changesLayout->addWidget(m_headerBar);
    auto *changesSplitter = new QSplitter(Qt::Vertical);
    changesSplitter->addWidget(m_gitStatusTree);
    changesSplitter->addWidget(commitContainer);
    changesSplitter->setStretchFactor(0, 1);
    changesSplitter->setStretchFactor(1, 0);
    changesLayout->addWidget(changesSplitter);
    m_sidebarTabs->addTab(changesTab, "Changes");

    // Tab 2 — History (commit list + commit files in a vertical splitter)
    auto *historyTab = new QWidget();
    auto *historyLayout = new QVBoxLayout(historyTab);
    historyLayout->setContentsMargins(0, 0, 0, 0);
    historyLayout->setSpacing(0);

    auto *historySplitter = new QSplitter(Qt::Vertical);

    m_commitHistoryList = new QListWidget();
    m_commitHistoryList->setAlternatingRowColors(true);
    m_commitHistoryList->setItemDelegate(new CommitDelegate(m_commitHistoryList));
    historySplitter->addWidget(m_commitHistoryList);

    // Commit files panel (hidden by default, inside history tab)
    m_commitFilesHeader = new QWidget();
    auto *cfHeaderLayout = new QHBoxLayout(m_commitFilesHeader);
    cfHeaderLayout->setContentsMargins(6, 2, 6, 2);
    auto *cfTitle = new QLabel("Commit Files");
    cfTitle->setStyleSheet("font-weight: bold;");
    auto *cfCloseBtn = new QPushButton(QStringLiteral("\u2716"));
    cfCloseBtn->setFixedSize(20, 20);
    cfCloseBtn->setFlat(true);
    cfCloseBtn->setStyleSheet("QPushButton { border: none; color: #8b949e; }"
                               "QPushButton:hover { color: #e6edf3; }");
    cfHeaderLayout->addWidget(cfTitle, 1);
    cfHeaderLayout->addWidget(cfCloseBtn, 0, Qt::AlignRight);
    m_commitFilesHeader->setVisible(false);

    m_commitFilesList = new QListWidget();
    m_commitFilesList->setVisible(false);
    m_commitFilesList->setStyleSheet(
        "QListWidget::item:selected {"
        "  background: palette(highlight);"
        "  color: palette(highlighted-text);"
        "}"
        "QListWidget::item:selected:!active {"
        "  background: palette(highlight);"
        "  color: palette(highlighted-text);"
        "}"
    );

    // Wrap header + list in a container so the splitter handle resizes
    // only the list, leaving the header at its fixed height.
    auto *cfContainer = new QWidget();
    auto *cfContainerLayout = new QVBoxLayout(cfContainer);
    cfContainerLayout->setContentsMargins(0, 0, 0, 0);
    cfContainerLayout->setSpacing(0);
    cfContainerLayout->addWidget(m_commitFilesHeader);
    cfContainerLayout->addWidget(m_commitFilesList, 1);
    cfContainer->setVisible(false);
    m_commitFilesContainer = cfContainer;

    historySplitter->addWidget(m_commitFilesContainer);
    historySplitter->setStretchFactor(0, 1);
    historySplitter->setStretchFactor(1, 1);

    historyLayout->addWidget(historySplitter);
    m_sidebarTabs->addTab(historyTab, "History");

    connect(cfCloseBtn, &QPushButton::clicked, this, [this]() {
        m_commitFilesHeader->setVisible(false);
        m_commitFilesList->setVisible(false);
        if (m_viewCommitFilesAction)
            m_viewCommitFilesAction->setChecked(false);
    });

    // Main horizontal splitter (sidebar + viewer)
    auto *splitter = new QSplitter(Qt::Horizontal);
    splitter->addWidget(m_sidebarTabs);
    splitter->addWidget(m_viewerStack);
    splitter->setStretchFactor(0, 1);
    splitter->setStretchFactor(1, 2);

    // Drop shadow for overlay effect
    auto *shadow = new QGraphicsDropShadowEffect();
    shadow->setBlurRadius(12);
    shadow->setOffset(2, 0);
    shadow->setColor(QColor(0, 0, 0, 100));
    m_recentDrawer->setGraphicsEffect(shadow);

    // Content area: only the main splitter (drawer overlays on top)
    auto *contentLayout = new QHBoxLayout();
    contentLayout->setContentsMargins(0, 0, 0, 0);
    contentLayout->setSpacing(0);
    contentLayout->addWidget(splitter, 1);

    mainLayout->addLayout(topLayout);
    mainLayout->addLayout(contentLayout, 1);

    setCentralWidget(centralWidget);

    // Processes
    m_gitProcess = new QProcess(this);
    connect(m_gitProcess, &QProcess::finished,
            this, &MainWindow::onGitProcessFinished);
    connect(m_gitProcess, &QProcess::errorOccurred,
            this, &MainWindow::onGitProcessErrorOccurred);

    m_commitProcess = new QProcess(this);
    connect(m_commitProcess, &QProcess::finished,
            this, &MainWindow::onCommitFinished);
    connect(m_commitProcess, &QProcess::errorOccurred,
            this, &MainWindow::onCommitErrorOccurred);

    m_branchProcess = new QProcess(this);
    connect(m_branchProcess, &QProcess::finished,
            this, &MainWindow::onBranchesLoaded);

    m_checkoutProcess = new QProcess(this);
    connect(m_checkoutProcess, &QProcess::finished,
            this, &MainWindow::onCheckoutFinished);
    connect(m_checkoutProcess, &QProcess::errorOccurred,
            this, &MainWindow::onCheckoutErrorOccurred);

    m_logProcess = new QProcess(this);
    connect(m_logProcess, &QProcess::finished,
            this, &MainWindow::onLogFinished);

    m_commitDetailProcess = new QProcess(this);
    connect(m_commitDetailProcess, &QProcess::finished,
            this, &MainWindow::onCommitDetailFinished);

    m_createBranchProcess = new QProcess(this);
    connect(m_createBranchProcess, &QProcess::finished,
            this, &MainWindow::onBranchesLoaded);

    m_installProcess = new QProcess(this);
    connect(m_installProcess, &QProcess::finished,
            this, &MainWindow::onInstallFinished);

    m_authProcess = new QProcess(this);
    connect(m_authProcess, &QProcess::finished,
            this, &MainWindow::onAuthCheckFinished);

    m_pushProcess = new QProcess(this);
    connect(m_pushProcess, &QProcess::finished,
            this, &MainWindow::onPushFinished);
    connect(m_pushProcess, &QProcess::errorOccurred,
            this, &MainWindow::onPushErrorOccurred);

    m_stageProcess = new QProcess(this);
    connect(m_stageProcess, &QProcess::finished,
            this, [this](int ec, QProcess::ExitStatus es) {
        if (es == QProcess::NormalExit && ec == 0)
            startGitStatusQuery();
    });

    // Connections
    connect(m_branchComboBox, &QComboBox::activated,
            this, &MainWindow::onBranchChanged);
    connect(m_deleteBranchButton, &QPushButton::clicked,
            this, &MainWindow::onDeleteBranch);
    connect(m_projectButton, &QPushButton::clicked,
            this, &MainWindow::onProjectButtonClicked);
    connect(m_openProjectButton, &QPushButton::clicked,
            this, &MainWindow::onProjectButtonClicked);
    connect(m_recentList, &QListWidget::itemClicked,
            this, &MainWindow::onRecentProjectClicked);
    connect(m_gitStatusTree, &QTreeWidget::itemClicked,
            this, &MainWindow::onTreeItemClicked);
    connect(m_commitHistoryList, &QListWidget::itemClicked,
            this, &MainWindow::onHistoryItemClicked);
    connect(m_commitFilesList, &QListWidget::itemClicked,
            this, &MainWindow::onCommitFileClicked);
    connect(m_summaryInput, &QLineEdit::textChanged,
            this, &MainWindow::onSummaryTextChanged);
    connect(m_commitButton, &QPushButton::clicked,
            this, &MainWindow::onCommitClicked);
    connect(m_pushButton, &QPushButton::clicked,
            this, &MainWindow::onPushClicked);

    connect(m_gitStatusTree, &QTreeWidget::customContextMenuRequested,
            this, &MainWindow::onTreeContextMenu);
    connect(m_selectAllCheck, &QCheckBox::checkStateChanged, this, [this](Qt::CheckState state) {
        setAllCheckStates(state);
    });

    m_fsWatcher = new QFileSystemWatcher(this);
    m_refreshTimer = new QTimer(this);
    m_refreshTimer->setSingleShot(true);
    m_refreshTimer->setInterval(500);
    connect(m_fsWatcher, &QFileSystemWatcher::directoryChanged,
            this, &MainWindow::onRepoDirChanged);
    connect(m_fsWatcher, &QFileSystemWatcher::fileChanged,
            this, &MainWindow::onRepoDirChanged);
    connect(m_refreshTimer, &QTimer::timeout,
            this, &MainWindow::onRefreshDebounce);
}

// --- Git config helpers ---

static QString runGitConfig(const QString &key)
{
    QProcess proc;
    proc.start("git", {"config", "--global", key});
    if (!proc.waitForFinished(3000) || proc.exitCode() != 0)
        return {};
    return QString::fromUtf8(proc.readAllStandardOutput()).trimmed();
}

static void runGitConfigSet(const QString &key, const QString &value)
{
    QProcess proc;
    proc.start("git", {"config", "--global", key, value});
    proc.waitForFinished(3000);
}

static QString gitRemoteOwner(const QString &path)
{
    QProcess proc;
    proc.setWorkingDirectory(path);
    proc.start("git", {"config", "--local", "remote.origin.url"});
    if (!proc.waitForFinished(3000) || proc.exitCode() != 0)
        return {};

    QString url = QString::fromUtf8(proc.readAllStandardOutput()).trimmed();
    if (url.isEmpty())
        return {};

    if (url.endsWith(".git"))
        url.chop(4);

    // Normalize SSH: git@github.com:owner/repo → ssh://git@github.com/owner/repo
    if (url.startsWith("git@"))
        url.replace(':', '/').prepend("ssh://");

    // Extract the path segment right after the hostname
    QRegularExpression re("^[a-z]+://[^@]*@?[^/]+/([^/]+)");
    auto m = re.match(url);
    if (m.hasMatch())
        return m.captured(1);

    return {};
}

// --- Recent projects persistence ---

static QString projectsFilePath()
{
#ifdef Q_OS_WIN
    return QString("C:/Users/%1/vomlabs/lazydesktop/projects.yaml")
        .arg(qEnvironmentVariable("USERNAME"));
#else
    return QDir::homePath() + "/vomlabs/lazydesktop/projects.yaml";
#endif
}

void MainWindow::loadRecentProjects()
{
    const QString path = projectsFilePath();
    if (!QFileInfo::exists(path))
        return;

    try {
        YAML::Node root = YAML::LoadFile(path.toStdString());
        const auto &projects = root["projects"];
        if (!projects || !projects.IsSequence())
            return;

        m_recentProjects.clear();
        for (size_t i = 0; i < projects.size(); ++i)
            m_recentProjects.append(
                QString::fromStdString(projects[i].as<std::string>()));
    } catch (...) {
        // corrupt file — start fresh
        m_recentProjects.clear();
    }

    populateRecentList();
}

void MainWindow::saveRecentProjects()
{
    const QString path = projectsFilePath();
    QDir().mkpath(QFileInfo(path).absolutePath());

    YAML::Emitter out;
    out << YAML::BeginMap;
    out << YAML::Key << "projects";
    out << YAML::Value << YAML::BeginSeq;
    for (const QString &p : m_recentProjects)
        out << p.toStdString();
    out << YAML::EndSeq;
    out << YAML::EndMap;

    QFile file(path);
    if (file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        file.write(out.c_str());
}

void MainWindow::addRecentProject(const QString &path)
{
    m_recentProjects.removeAll(path);
    m_recentProjects.prepend(path);
    while (m_recentProjects.size() > kMaxRecentProjects)
        m_recentProjects.removeLast();
    saveRecentProjects();
    populateRecentList();
}

void MainWindow::populateRecentList()
{
    m_recentList->clear();

    // Group projects by remote owner
    QMap<QString, QStringList> groups;
    QStringList ungrouped;

    for (const QString &path : m_recentProjects) {
        QString owner = gitRemoteOwner(path);
        if (owner.isEmpty())
            ungrouped.append(path);
        else
            groups[owner].append(path);
    }

    // Sort projects within each group
    auto sortByDirName = [](QStringList &paths) {
        std::sort(paths.begin(), paths.end(), [](const QString &a, const QString &b) {
            return QString::localeAwareCompare(QDir(a).dirName(), QDir(b).dirName()) < 0;
        });
    };
    for (auto it = groups.begin(); it != groups.end(); ++it)
        sortByDirName(it.value());
    sortByDirName(ungrouped);

    // Build sorted owner list
    QStringList owners = groups.keys();
    std::sort(owners.begin(), owners.end(), [](const QString &a, const QString &b) {
        return QString::localeAwareCompare(a, b) < 0;
    });

    // Lambda to create a project row widget
    auto addProjectRow = [this](const QString &path) {
        auto *item = new QListWidgetItem();
        item->setData(Qt::UserRole, path);
        m_recentList->addItem(item);

        auto *row = new QWidget();
        auto *layout = new QHBoxLayout(row);
        layout->setContentsMargins(4, 2, 4, 2);
        layout->setSpacing(4);

        auto *dirtyDot = new QLabel();
        dirtyDot->setFixedSize(8, 8);
        if (isDirtyRepository(path)) {
            dirtyDot->setStyleSheet("background: #f0c000; border-radius: 4px;");
            dirtyDot->setToolTip("Uncommitted changes");
        }

        auto *label = new QPushButton(QDir(path).dirName());
        label->setToolTip(path);
        label->setCursor(Qt::PointingHandCursor);
        label->setFlat(true);
        label->setStyleSheet("QPushButton { text-align: left; border: none; padding: 0; }");

        auto *btn = new QPushButton(QStringLiteral("\u2716"));
        btn->setFixedSize(22, 22);
        btn->setCursor(Qt::PointingHandCursor);
        btn->setFlat(true);
        btn->setProperty("repoPath", path);

        connect(label, &QPushButton::clicked, this, [this, path]() {
            m_recentDrawer->setVisible(false);
            openRepository(path);
        });
        connect(btn, &QPushButton::clicked, this, &MainWindow::onRemoveRecentProject);

        layout->addWidget(dirtyDot);
        layout->addWidget(label, 1);
        layout->addWidget(btn, 0, Qt::AlignRight);
        row->setLayout(layout);

        m_recentList->setItemWidget(item, row);
    };

    auto addCategoryRow = [this](const QString &title) {
        auto *item = new QListWidgetItem();
        item->setFlags(item->flags() & ~Qt::ItemIsSelectable);
        m_recentList->addItem(item);

        auto *row = new QWidget();
        auto *layout = new QHBoxLayout(row);
        layout->setContentsMargins(4, 4, 4, 2);
        auto *label = new QLabel(title);
        QFont f = label->font();
        f.setBold(true);
        f.setPointSize(f.pointSize() - 1);
        label->setFont(f);
        label->setStyleSheet("color: #888;");
        layout->addWidget(label);
        m_recentList->setItemWidget(item, row);
    };

    // Render grouped projects
    for (const QString &owner : owners) {
        addCategoryRow(owner);
        for (const QString &path : groups[owner])
            addProjectRow(path);
    }

    // Ungrouped projects at the bottom
    if (!ungrouped.isEmpty()) {
        addCategoryRow("Other");
        for (const QString &path : ungrouped)
            addProjectRow(path);
    }
}

void MainWindow::onRemoveRecentProject()
{
    auto *btn = qobject_cast<QPushButton *>(sender());
    if (!btn)
        return;

    const QString path = btn->property("repoPath").toString();
    if (path.isEmpty())
        return;

    QMessageBox msg(this);
    msg.setWindowTitle("Remove Project");
    msg.setText(QString("Remove \"%1\" from the list?").arg(QDir(path).dirName()));
    msg.setInformativeText("You can also delete the project directory.");
    auto *removeBtn = msg.addButton("Remove from List", QMessageBox::AcceptRole);
    auto *deleteBtn = msg.addButton("Yes, and Delete Directory", QMessageBox::DestructiveRole);
    msg.addButton(QMessageBox::Cancel);
    msg.setDefaultButton(QMessageBox::Cancel);

    msg.exec();

    if (msg.clickedButton() == removeBtn) {
        m_recentProjects.removeAll(path);
        saveRecentProjects();
        populateRecentList();
        if (path == m_repoPath)
            closeRepository();
    } else if (msg.clickedButton() == deleteBtn) {
        auto really = QMessageBox::question(this, "Confirm Deletion",
            QString("Are you really sure you want to permanently delete\n\"%1\"?\n\n"
                    "This cannot be undone.").arg(path),
            QMessageBox::Yes | QMessageBox::No, QMessageBox::No);

        if (really == QMessageBox::Yes) {
            m_recentProjects.removeAll(path);
            saveRecentProjects();
            populateRecentList();
            if (path == m_repoPath)
                closeRepository();

            QDir dir(path);
            if (dir.exists())
                dir.removeRecursively();
        }
    }
}

void MainWindow::onScanFolder()
{
    const QString dir = QFileDialog::getExistingDirectory(
        this, "Select Folder to Scan for Git Repositories");
    if (dir.isEmpty())
        return;

    QDir rootDir(dir);
    const QStringList subdirs = rootDir.entryList(QDir::Dirs | QDir::NoDotAndDotDot, QDir::Name);

    int added = 0;
    int skipped = 0;
    for (const QString &subdir : subdirs) {
        const QString fullPath = rootDir.filePath(subdir);
        if (isGitRepository(fullPath)) {
            m_recentProjects.removeAll(fullPath);
            m_recentProjects.prepend(fullPath);
            added++;
        } else {
            skipped++;
        }
    }

    if (added == 0 && skipped == 0) {
        QMessageBox::information(this, "No Subdirectories Found",
            "The selected folder has no subdirectories.");
        return;
    }

    saveRecentProjects();
    populateRecentList();
    m_recentDrawer->setVisible(false);

    QMessageBox::information(this, "Scan Complete",
        QString("Added %1 git project(s) to the list.\n"
                "Skipped %2 folder(s) without a .git directory.")
            .arg(added).arg(skipped));
}

// --- Project button & drawer ---

void MainWindow::onProjectButtonClicked()
{
    auto *btn = qobject_cast<QPushButton *>(sender());
    if (!btn)
        return;

    // "Open Project" button inside the drawer always opens folder dialog
    if (btn == m_openProjectButton) {
        const QString dir = QFileDialog::getExistingDirectory(
            this, "Open Git Repository");
        if (!dir.isEmpty())
            openRepository(dir);
        return;
    }

    // m_projectButton behaviour depends on state
    if (m_repoPath.isEmpty()) {
        // No project open → open folder dialog
        const QString dir = QFileDialog::getExistingDirectory(
            this, "Open Git Repository");
        if (!dir.isEmpty())
            openRepository(dir);
    } else {
        // Project open → toggle overlay drawer
        if (m_recentDrawer->isVisible()) {
            m_recentDrawer->hide();
        } else {
            auto *cw = centralWidget();
            int toolbarH = m_projectButton->mapTo(cw, QPoint(0, 0)).y()
                         + m_projectButton->height() + 2;
            m_recentDrawer->setGeometry(0, toolbarH, 260,
                cw->height() - toolbarH);
            m_recentDrawer->raise();
            m_recentDrawer->show();
        }
    }
}

void MainWindow::onRecentProjectClicked(QListWidgetItem *item)
{
    if (!item)
        return;

    const QString path = item->data(Qt::UserRole).toString();
    if (path.isEmpty())
        return;

    m_recentDrawer->setVisible(false);
    openRepository(path);
}

// --- Open / validate repository ---

bool MainWindow::openRepository(const QString &path)
{
    if (!isGitRepository(path)) {
        QMessageBox::warning(this, "Invalid Folder",
            "The selected folder is not a Git repository.\n"
            "Please select a folder that contains a .git directory.");
        return false;
    }

    m_repoPath = path;
    m_projectButton->setText(QDir(path).dirName());
    m_currentPathLabel->setVisible(false);
    m_pushButton->setEnabled(true);
    m_gitStatusTree->clear();
    m_fileContentViewer->clear();
    m_viewerStack->setCurrentIndex(0);
    m_recentDrawer->setVisible(false);

    addRecentProject(path);
    loadBranches();
    startGitStatusQuery();
    startGitLogQuery();

    // Watch for file changes to auto-refresh
    m_fsWatcher->removePaths(m_fsWatcher->files());
    m_fsWatcher->removePaths(m_fsWatcher->directories());
    const QString gitDir = QDir(path).filePath(".git");
    m_fsWatcher->addPath(gitDir);
    m_fsWatcher->addPath(QDir(gitDir).filePath("index"));
    m_fsWatcher->addPath(QDir(gitDir).filePath("HEAD"));

    return true;
}

void MainWindow::closeRepository()
{
    if (m_repoPath.isEmpty())
        return;

    m_repoPath.clear();
    m_projectButton->setText("Open Folder");
    m_currentPathLabel->setVisible(false);
    m_gitStatusTree->clear();
    m_commitHistoryList->clear();
    m_commitFilesList->clear();
    m_commitFilesList->setVisible(false);
    m_fileContentViewer->clear();
    m_viewerStack->setCurrentIndex(1);
    m_summaryInput->clear();
    m_descriptionInput->clear();
    m_commitButton->setEnabled(false);
    m_pushButton->setEnabled(false);
    m_branchComboBox->setEnabled(false);
    m_branchComboBox->clear();
    m_deleteBranchButton->setEnabled(false);
    m_openGitHubAction->setEnabled(false);
    m_currentBranch.clear();
    m_selectedCommitHash.clear();
    m_fsWatcher->removePaths(m_fsWatcher->files());
    m_fsWatcher->removePaths(m_fsWatcher->directories());
}

void MainWindow::onOpenEditor()
{
    if (m_repoPath.isEmpty())
        return;
    QProcess::startDetached("kate", {m_repoPath});
}

void MainWindow::onOpenFileManager()
{
    if (m_repoPath.isEmpty())
        return;
    QDesktopServices::openUrl(QUrl::fromLocalFile(m_repoPath));
}

void MainWindow::onOpenTerminal()
{
    if (m_repoPath.isEmpty())
        return;
    QProcess::startDetached("konsole", {"--workdir", m_repoPath});
}

void MainWindow::onOpenGitHub()
{
    if (m_repoPath.isEmpty())
        return;

    auto *proc = new QProcess(this);
    proc->setWorkingDirectory(m_repoPath);
    proc->start("git", {"remote", "get-url", "origin"});

    connect(proc, &QProcess::finished, this, [proc](int ec, QProcess::ExitStatus es) {
        proc->deleteLater();
        if (es != QProcess::NormalExit || ec != 0)
            return;

        QString url = QString::fromUtf8(proc->readAllStandardOutput()).trimmed();

        // Convert SSH to HTTPS
        if (url.startsWith("git@")) {
            url.remove(0, 4); // "git@"
            url.replace(':', '/');
            url.prepend("https://");
        }
        // Remove trailing .git
        if (url.endsWith(".git"))
            url.chop(4);

        if (!url.isEmpty())
            QDesktopServices::openUrl(QUrl(url));
    });
}

void MainWindow::onOpenSettings()
{
    QDialog dialog(this);
    dialog.setWindowTitle("Settings");
    dialog.setMinimumSize(500, 350);

    auto *layout = new QHBoxLayout(&dialog);

    auto *categories = new QListWidget();
    categories->setFixedWidth(120);
    categories->addItem("General");
    categories->addItem("Appearance");
    categories->addItem("Git");

    auto *stack = new QStackedWidget();

    // General page
    auto *generalPage = new QWidget();
    auto *generalLayout = new QVBoxLayout(generalPage);
    auto *generalPlaceholder = new QLabel("General settings coming soon.");
    generalPlaceholder->setAlignment(Qt::AlignCenter);
    generalPlaceholder->setStyleSheet("color: gray;");
    generalLayout->addWidget(generalPlaceholder);

    // Appearance page
    auto *appearancePage = new QWidget();
    auto *appearanceLayout = new QVBoxLayout(appearancePage);
    auto *appearancePlaceholder = new QLabel("Appearance settings coming soon.");
    appearancePlaceholder->setAlignment(Qt::AlignCenter);
    appearancePlaceholder->setStyleSheet("color: gray;");
    appearanceLayout->addWidget(appearancePlaceholder);

    // Git page
    auto *gitPage = new QWidget();
    auto *gitLayout = new QFormLayout(gitPage);
    gitLayout->setContentsMargins(12, 12, 12, 12);
    gitLayout->setSpacing(8);

    auto *gitNameInput = new QLineEdit();
    gitNameInput->setPlaceholderText("Your Name");
    QString storedName = runGitConfig("user.name");
    if (!storedName.isEmpty())
        gitNameInput->setText(storedName);

    auto *gitEmailInput = new QLineEdit();
    gitEmailInput->setPlaceholderText("you@example.com");
    QString storedEmail = runGitConfig("user.email");
    if (!storedEmail.isEmpty())
        gitEmailInput->setText(storedEmail);

    gitLayout->addRow("User Name:", gitNameInput);
    gitLayout->addRow("User Email:", gitEmailInput);

    auto *gitInfoLabel = new QLabel("These values are read from and saved to global git config.");
    gitInfoLabel->setStyleSheet("color: gray; font-size: 11px;");
    gitInfoLabel->setWordWrap(true);
    gitLayout->addRow(gitInfoLabel);

    // Save on accept
    connect(&dialog, &QDialog::accepted, this, [gitNameInput, gitEmailInput]() {
        const QString name = gitNameInput->text().trimmed();
        const QString email = gitEmailInput->text().trimmed();
        if (!name.isEmpty())
            runGitConfigSet("user.name", name);
        if (!email.isEmpty())
            runGitConfigSet("user.email", email);
    });

    stack->addWidget(generalPage);
    stack->addWidget(appearancePage);
    stack->addWidget(gitPage);

    connect(categories, &QListWidget::currentRowChanged, stack, &QStackedWidget::setCurrentIndex);

    auto *rightPanel = new QWidget();
    auto *rightLayout = new QVBoxLayout(rightPanel);
    rightLayout->addWidget(stack, 1);

    auto *buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);
    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);
    rightLayout->addWidget(buttons);

    layout->addWidget(categories);
    layout->addWidget(rightPanel, 1);

    categories->setCurrentRow(0);

    dialog.exec();
}

bool MainWindow::isGitRepository(const QString &path)
{
    const QFileInfo gitInfo(QDir(path).filePath(".git"));
    return gitInfo.exists();
}

bool MainWindow::isDirtyRepository(const QString &path)
{
    if (!QDir(path).exists())
        return false;
    QProcess proc;
    proc.setWorkingDirectory(path);
    proc.start("git", {"status", "--porcelain"});
    if (!proc.waitForFinished(3000) || proc.exitCode() != 0)
        return false;
    return !proc.readAllStandardOutput().trimmed().isEmpty();
}

static QIcon statusIcon(const QColor &color)
{
    QPixmap pm(10, 10);
    pm.fill(color);
    return QIcon(pm);
}

void MainWindow::addGitFileToTree(const QString &path, const QString &prefix)
{
    auto *fileItem = new QTreeWidgetItem();
    fileItem->setText(0, path);
    fileItem->setData(0, Qt::UserRole, path);

    fileItem->setFlags(fileItem->flags() | Qt::ItemIsUserCheckable);
    fileItem->setCheckState(0, Qt::Checked);

    const QChar status = prefix.trimmed().isEmpty() ? QChar() : prefix.trimmed().at(0);
    fileItem->setData(0, Qt::UserRole + 1, status);

    switch (status.toLatin1()) {
    case 'M': fileItem->setIcon(0, statusIcon(QColor("#f0c000"))); break;
    case 'D': fileItem->setIcon(0, statusIcon(QColor("#e04040"))); break;
    case 'A': fileItem->setIcon(0, statusIcon(QColor("#40c040"))); break;
    case 'R': fileItem->setIcon(0, statusIcon(QColor("#c080ff"))); break;
    case '?': fileItem->setIcon(0, statusIcon(QColor("#c0c0c0"))); break;
    default:  break;
    }

    m_gitStatusTree->addTopLevelItem(fileItem);
}

// --- View menu toggles ---

void MainWindow::toggleCommitPanel(bool visible)
{
    if (m_commitContainer)
        m_commitContainer->setVisible(visible);
}

void MainWindow::toggleCommitFilesPanel(bool visible)
{
    if (m_commitFilesHeader)
        m_commitFilesHeader->setVisible(visible);
    if (m_commitFilesList)
        m_commitFilesList->setVisible(visible);
}

// --- Checkbox helpers ---

void MainWindow::setAllCheckStates(Qt::CheckState state)
{
    QTreeWidgetItemIterator it(m_gitStatusTree);
    while (*it) {
        (*it)->setCheckState(0, state);
        ++it;
    }
}

QStringList MainWindow::checkedFiles() const
{
    QStringList files;
    QTreeWidgetItemIterator it(m_gitStatusTree);
    while (*it) {
        if ((*it)->checkState(0) == Qt::Checked)
            files << (*it)->data(0, Qt::UserRole).toString();
        ++it;
    }
    return files;
}

// --- Tree context menu (Discard) ---

void MainWindow::onTreeContextMenu(const QPoint &pos)
{
    auto *item = m_gitStatusTree->itemAt(pos);
    if (!item)
        return;

    const QString path = item->data(0, Qt::UserRole).toString();
    if (path.isEmpty())
        return;

    const QChar status = item->data(0, Qt::UserRole + 1).toChar();
    const bool isUntracked = (status == '?');

    QMenu menu(this);
    if (isUntracked) {
        auto *deleteAct = menu.addAction("Delete File");
        connect(deleteAct, &QAction::triggered, this, [this, path]() {
            QFile::remove(QDir(m_repoPath).filePath(path));
            startGitStatusQuery();
        });
    } else {
        auto *discardAct = menu.addAction("Discard Changes");
        connect(discardAct, &QAction::triggered, this, [this, path]() {
            m_stageProcess->setWorkingDirectory(m_repoPath);
            m_stageProcess->start("git", {"restore", path});
        });
    }

    menu.exec(m_gitStatusTree->viewport()->mapToGlobal(pos));
}

void MainWindow::onDiscardFile()
{
    auto *btn = qobject_cast<QPushButton *>(sender());
    if (!btn)
        return;

    const QString path = btn->property("filePath").toString();
    if (path.isEmpty())
        return;

    m_stageProcess->setWorkingDirectory(m_repoPath);
    m_stageProcess->start("git", {"restore", path});
}

// --- Tree item click (diff viewer) ---

static QString runGitDiff(const QString &repoPath, const QStringList &args)
{
    QProcess proc;
    proc.setWorkingDirectory(repoPath);
    proc.start("git", args);
    if (!proc.waitForFinished(5000) || proc.exitCode() != 0)
        return {};
    return QString::fromUtf8(proc.readAllStandardOutput());
}

void MainWindow::onTreeItemClicked(QTreeWidgetItem *item, int column)
{
    Q_UNUSED(column);

    if (!item)
        return;

    const QString relPath = item->data(0, Qt::UserRole).toString();
    if (relPath.isEmpty())
        return;

    m_viewerStack->setCurrentIndex(0);

    QString diff = runGitDiff(m_repoPath, {"diff", "HEAD", "--", relPath});

    if (diff.isEmpty())
        diff = runGitDiff(m_repoPath, {"diff", "@{u}..HEAD", "--", relPath});

    if (diff.isEmpty()) {
        m_fileContentViewer->clear();
        return;
    }

    m_fileContentViewer->setDiff(diff);
}

// --- Commit pane ---

void MainWindow::onSummaryTextChanged(const QString &text)
{
    m_commitButton->setEnabled(!text.trimmed().isEmpty());
}

void MainWindow::onCommitClicked()
{
    const QStringList files = checkedFiles();
    if (files.isEmpty()) {
        QMessageBox::information(this, "Nothing Selected",
            "Check at least one file to commit.");
        return;
    }

    if (!m_commitProcess || m_commitProcess->state() != QProcess::NotRunning) {
        if (m_commitProcess) {
            m_commitProcess->deleteLater();
            m_commitProcess = nullptr;
        }
        m_commitProcess = new QProcess(this);
        connect(m_commitProcess, &QProcess::finished,
                this, &MainWindow::onCommitFinished);
        connect(m_commitProcess, &QProcess::errorOccurred,
                this, &MainWindow::onCommitErrorOccurred);
    }

    // Stage all checked files first, then commit
    auto *addProc = new QProcess(this);
    addProc->setWorkingDirectory(m_repoPath);
    QStringList addArgs = {"add", "--"};
    addArgs.append(files);
    addProc->start("git", addArgs);

    m_commitButton->setEnabled(false);
    m_commitButton->setText("Staging…");

    connect(addProc, &QProcess::finished, this, [this, addProc](int ec, QProcess::ExitStatus es) {
        addProc->deleteLater();
        m_commitButton->setText("Commit");

        if (es != QProcess::NormalExit || ec != 0) {
            const QString err = QString::fromUtf8(addProc->readAllStandardError());
            QMessageBox::warning(this, "Stage Failed", err);
            m_commitButton->setEnabled(!m_summaryInput->text().trimmed().isEmpty());
            return;
        }

        QStringList args = {"commit", "-m", m_summaryInput->text().trimmed()};

        const QString desc = m_descriptionInput->toPlainText().trimmed();
        if (!desc.isEmpty())
            args << "-m" << desc;

        m_commitProcess->setWorkingDirectory(m_repoPath);
        m_commitProcess->start("git", args);
    });
}

void MainWindow::onCommitFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit) {
        m_commitButton->setEnabled(!m_summaryInput->text().trimmed().isEmpty());
        return;
    }

    if (exitCode != 0) {
        const QString stderrOut = QString::fromUtf8(
            m_commitProcess->readAllStandardError());
        QMessageBox::warning(this, "Commit Failed", stderrOut);

        m_commitProcess->deleteLater();
        m_commitProcess = nullptr;

        m_commitButton->setEnabled(!m_summaryInput->text().trimmed().isEmpty());
        return;
    }

    m_summaryInput->clear();
    m_descriptionInput->clear();
    m_commitButton->setEnabled(false);
    m_commitButton->setText("Commit");

    m_gitStatusTree->clear();
    m_fileContentViewer->clear();
    startGitStatusQuery();
    startGitLogQuery();
}

void MainWindow::onCommitErrorOccurred(QProcess::ProcessError error)
{
    if (error == QProcess::FailedToStart) {
        QMessageBox::critical(this, "Git Not Found",
            "Git is not installed or not available on the system PATH.");
    }

    if (!m_commitProcess)
        return;
    m_commitProcess->deleteLater();
    m_commitProcess = nullptr;
    m_commitButton->setEnabled(!m_summaryInput->text().trimmed().isEmpty());
}

// --- Branch switching ---

void MainWindow::loadBranches()
{
    if (m_branchProcess->state() == QProcess::Running)
        return;

    m_branchComboBox->setEnabled(false);
    m_populatingBranches = true;
    m_branchComboBox->clear();

    // Get current branch synchronously (fast)
    QProcess cur;
    cur.setWorkingDirectory(m_repoPath);
    cur.start("git", {"branch", "--show-current"});
    if (cur.waitForFinished(3000) && cur.exitCode() == 0)
        m_currentBranch = QString::fromUtf8(cur.readAllStandardOutput()).trimmed();

    // Load all branches asynchronously
    m_branchProcess->setWorkingDirectory(m_repoPath);
    m_branchProcess->start("git", {"branch", "-a", "--format=%(refname:short)"});
}

void MainWindow::onBranchesLoaded()
{
    const QString output = QString::fromUtf8(
        m_branchProcess->readAllStandardOutput());

    m_populatingBranches = true;
    m_branchComboBox->clear();

    // "Create New Branch..." sentinel at the top
    m_branchComboBox->addItem("+ Create New Branch...");
    m_branchComboBox->insertSeparator(1);

    QStringList locals, remotes;
    const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

    for (const QString &line : lines) {
        if (line.startsWith("remotes/"))
            remotes << line;
        else
            locals << line;
    }

    for (const QString &b : locals)
        m_branchComboBox->addItem(b);

    if (!remotes.isEmpty()) {
        m_branchComboBox->insertSeparator(m_branchComboBox->count());
        for (const QString &b : remotes)
            m_branchComboBox->addItem(b);
    }

    // Select current branch
    int idx = m_branchComboBox->findText(m_currentBranch);
    if (idx >= 0)
        m_branchComboBox->setCurrentIndex(idx);

    m_branchComboBox->setEnabled(true);
    m_populatingBranches = false;

    // Update delete button
    updateDeleteButtonState();
}

void MainWindow::onBranchChanged(int index)
{
    if (m_populatingBranches)
        return;

    const QString selected = m_branchComboBox->itemText(index);
    if (selected.isEmpty())
        return;

    // Handle "Create New Branch..." sentinel
    if (index == 0) {
        createNewBranch();
        return;
    }

    // Skip actual separators
    if (m_branchComboBox->itemData(index, Qt::AccessibleDescriptionRole).toString() == "separator")
        return;

    if (selected == m_currentBranch) {
        updateDeleteButtonState();
        return;
    }

    // Check for uncommitted changes
    QProcess dirty;
    dirty.setWorkingDirectory(m_repoPath);
    dirty.start("git", {"status", "--porcelain"});
    if (dirty.waitForFinished(3000) && dirty.exitCode() == 0) {
        const QString status = QString::fromUtf8(dirty.readAllStandardOutput()).trimmed();
        if (!status.isEmpty()) {
            auto reply = QMessageBox::question(this, "Uncommitted Changes",
                "You have uncommitted changes. Commit them before switching branches?\n\n"
                "Press 'Yes' to commit, 'No' to discard changes and switch, or 'Cancel' to abort.",
                QMessageBox::Yes | QMessageBox::No | QMessageBox::Cancel);

            if (reply == QMessageBox::Cancel) {
                restoreBranchSelection();
                return;
            }

            if (reply == QMessageBox::Yes) {
                m_summaryInput->setFocus();
                restoreBranchSelection();
                QMessageBox::information(this, "Commit First",
                    "Please write a summary above and click Commit, then switch branches.");
                return;
            }
        }
    }

    doCheckout(selected);
}

void MainWindow::doCheckout(const QString &branch)
{
    m_branchComboBox->setEnabled(false);
    m_checkoutProcess->setWorkingDirectory(m_repoPath);
    m_checkoutProcess->start("git", {"checkout", branch});
}

void MainWindow::restoreBranchSelection()
{
    m_populatingBranches = true;
    int idx = m_branchComboBox->findText(m_currentBranch);
    if (idx >= 0)
        m_branchComboBox->setCurrentIndex(idx);
    m_populatingBranches = false;
}

void MainWindow::onCheckoutFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    m_branchComboBox->setEnabled(true);

    if (exitStatus != QProcess::NormalExit)
        return;

    if (exitCode != 0) {
        const QString err = QString::fromUtf8(
            m_checkoutProcess->readAllStandardError());
        QMessageBox::warning(this, "Checkout Failed", err);
        restoreBranchSelection();
        return;
    }

    refreshAll();
}

void MainWindow::onCheckoutErrorOccurred(QProcess::ProcessError error)
{
    m_branchComboBox->setEnabled(true);

    if (error == QProcess::FailedToStart) {
        QMessageBox::critical(this, "Git Not Found",
            "Git is not installed or not available on the system PATH.");
    }

    restoreBranchSelection();
}

void MainWindow::updateDeleteButtonState()
{
    int idx = m_branchComboBox->currentIndex();
    if (idx <= 1) { // sentinel or separator
        m_deleteBranchButton->setEnabled(false);
        return;
    }

    const QString branch = m_branchComboBox->itemText(idx);
    m_deleteBranchButton->setEnabled(
        !branch.startsWith("remotes/") && branch != m_currentBranch);
}

void MainWindow::onDeleteBranch()
{
    int idx = m_branchComboBox->currentIndex();
    if (idx <= 1)
        return;

    const QString branch = m_branchComboBox->itemText(idx);
    if (branch.startsWith("remotes/") || branch == m_currentBranch)
        return;

    auto reply = QMessageBox::question(this, "Delete Branch",
        QString("Are you sure you want to delete the branch '%1'?").arg(branch),
        QMessageBox::Yes | QMessageBox::No);

    if (reply != QMessageBox::Yes)
        return;

    m_populatingBranches = true;
    QProcess del;
    del.setWorkingDirectory(m_repoPath);
    del.start("git", {"branch", "-D", branch});
    if (del.waitForFinished(5000) && del.exitCode() == 0) {
        loadBranches();
    } else {
        QMessageBox::warning(this, "Delete Failed",
            QString::fromUtf8(del.readAllStandardError()));
        m_populatingBranches = false;
    }
}

void MainWindow::createNewBranch()
{
    bool ok = false;
    const QString name = QInputDialog::getText(this,
        "Create Branch", "Branch name:", QLineEdit::Normal, {}, &ok);

    if (!ok || name.trimmed().isEmpty()) {
        restoreBranchSelection();
        return;
    }

    m_populatingBranches = true;
    m_createBranchProcess->setWorkingDirectory(m_repoPath);
    m_createBranchProcess->start("git", {"checkout", "-b", name.trimmed()});
}

void MainWindow::refreshAll()
{
    m_gitStatusTree->clear();
    m_fileContentViewer->clear();
    m_summaryInput->clear();
    m_descriptionInput->clear();
    m_commitButton->setEnabled(false);

    loadBranches();
    startGitStatusQuery();
}

// --- Push / Fetch / Pull ---

void MainWindow::onPushClicked()
{
    if (m_pushProcess->state() != QProcess::NotRunning)
        return;

    m_pushButton->setEnabled(false);
    m_pushProcess->setWorkingDirectory(m_repoPath);
    setupAuthEnv(m_pushProcess);

    switch (m_pushState) {
    case PushState::Push:
        m_pushProcess->start("git", {"push"});
        break;
    case PushState::Fetch:
        m_pushProcess->start("git", {"fetch"});
        break;
    case PushState::Pull:
        m_pushProcess->start("git", {"pull"});
        break;
    }
}

void MainWindow::onPushFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit) {
        m_pushButton->setEnabled(true);
        return;
    }

    const QString stdOut = QString::fromUtf8(
        m_pushProcess->readAllStandardOutput());
    const QString stdErr = QString::fromUtf8(
        m_pushProcess->readAllStandardError());

    switch (m_pushState) {
    case PushState::Push: {
        if (exitCode != 0) {
            if (stdErr.contains("rejected") || stdErr.contains("non-fast-forward")) {
                m_pushState = PushState::Pull;
                m_pushButton->setText("Pull");
                QMessageBox::warning(this, "Push Rejected",
                    "The remote has commits you don't have locally.\n"
                    "Pull first, then push again.");
            } else {
                QMessageBox::warning(this, "Push Failed", stdErr);
            }
            m_pushButton->setEnabled(true);
            return;
        }

        if (stdOut.contains("Everything up-to-date")) {
            m_pushState = PushState::Fetch;
            m_pushButton->setText("Fetch");
        } else {
            m_gitStatusTree->clear();
            m_fileContentViewer->clear();
            startGitStatusQuery();
        }
        m_pushButton->setEnabled(true);
        break;
    }
    case PushState::Fetch: {
        if (exitCode != 0) {
            QMessageBox::warning(this, "Fetch Failed", stdErr);
            m_pushState = PushState::Push;
            m_pushButton->setText("Push");
            m_pushButton->setEnabled(true);
            return;
        }

        QProcess behind;
        behind.setWorkingDirectory(m_repoPath);
        behind.start("git", {"rev-list", "--count", "HEAD..@{u}"});
        if (behind.waitForFinished(5000) && behind.exitCode() == 0) {
            int count = behind.readAllStandardOutput().trimmed().toInt();
            if (count > 0) {
                m_pushState = PushState::Pull;
                m_pushButton->setText("Pull");
            } else {
                m_pushState = PushState::Push;
                m_pushButton->setText("Push");
            }
        } else {
            m_pushState = PushState::Push;
            m_pushButton->setText("Push");
        }
        m_pushButton->setEnabled(true);
        break;
    }
    case PushState::Pull: {
        if (exitCode != 0) {
            QMessageBox::warning(this, "Pull Failed", stdErr);
            m_pushButton->setEnabled(true);
            return;
        }

        m_pushState = PushState::Push;
        m_pushButton->setText("Push");

        m_gitStatusTree->clear();
        m_fileContentViewer->clear();
        startGitStatusQuery();
        m_pushButton->setEnabled(true);
        break;
    }
    }
}

void MainWindow::onPushErrorOccurred(QProcess::ProcessError error)
{
    if (error == QProcess::FailedToStart) {
        QMessageBox::critical(this, "Git Not Found",
            "Git is not installed or not available on the system PATH.");
    }
    m_pushButton->setEnabled(true);
}

// --- Status queries ---

void MainWindow::startGitStatusQuery()
{
    if (m_gitProcess->state() != QProcess::NotRunning)
        m_gitProcess->kill();

    m_currentQuery = GitQuery::Status;
    m_gitProcess->setWorkingDirectory(m_repoPath);
    m_gitProcess->start("git", {"status", "--porcelain"});
}

void MainWindow::startGitUnpushedQuery()
{
    if (m_gitProcess->state() != QProcess::NotRunning)
        m_gitProcess->kill();

    m_currentQuery = GitQuery::Unpushed;
    m_gitProcess->setWorkingDirectory(m_repoPath);
    m_gitProcess->start("git", {"diff", "--name-only", "@{u}..HEAD"});
}

void MainWindow::onGitProcessFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit) {
        m_currentQuery = GitQuery::None;
        return;
    }

    if (exitCode != 0) {
        if (m_currentQuery == GitQuery::Status) {
            QMessageBox::warning(this, "Git Error",
                "Failed to query git status.\n"
                "Make sure the repository is valid.");
        }
        m_currentQuery = GitQuery::None;
        return;
    }

    const QString output = QString::fromUtf8(
        m_gitProcess->readAllStandardOutput());

    if (m_currentQuery == GitQuery::Status) {
        m_gitStatusTree->clear();
        const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

        for (const QString &line : lines) {
            QString prefix;
            const QString xy = line.left(2);

            if (xy == "??")
                prefix = "[?]";
            else if (xy.contains('M'))
                prefix = "[M]";
            else if (xy.contains('A'))
                prefix = "[A]";
            else if (xy.contains('D'))
                prefix = "[D]";
            else if (xy.contains('R'))
                prefix = "[R]";
            else
                prefix = "[*]";

            const QString path = line.mid(3).trimmed();
            addGitFileToTree(path, prefix);
        }

        m_changedFilesLabel->setText(
            QString("%1 changed files").arg(m_gitStatusTree->topLevelItemCount()));
        startGitUnpushedQuery();
    } else if (m_currentQuery == GitQuery::Unpushed) {
        const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

        for (const QString &line : lines) {
            const QString trimmed = line.trimmed();
            if (trimmed.isEmpty())
                continue;

            addGitFileToTree(trimmed, "[P]");
        }

        m_changedFilesLabel->setText(
            QString("%1 changed files").arg(m_gitStatusTree->topLevelItemCount()));
        m_currentQuery = GitQuery::None;
    }
}

void MainWindow::onGitProcessErrorOccurred(QProcess::ProcessError error)
{
    if (error == QProcess::FailedToStart) {
        QMessageBox::critical(this, "Git Not Found",
            "Git is not installed or not available on the system PATH.\n"
            "Please install Git to use this feature.");
    }

    m_currentQuery = GitQuery::None;
}

// --- Git log (History tab) ---

void MainWindow::startGitLogQuery()
{
    if (m_repoPath.isEmpty())
        return;

    m_logProcess->setWorkingDirectory(m_repoPath);
    m_logProcess->start("git", {
        "log", "--pretty=format:%h%n%an%n%ar%n%s", "--max-count=100",
        "--abbrev-commit"
    });
}

void MainWindow::onLogFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit || exitCode != 0)
        return;

    m_commitHistoryList->clear();

    const QString output = QString::fromUtf8(m_logProcess->readAllStandardOutput());
    const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

    // Format is groups of 4 lines: hash, author, date, subject
    for (int i = 0; i + 3 < lines.size(); i += 4) {
        const QString hash  = lines[i];
        const QString author = lines[i + 1];
        const QString date  = lines[i + 2];
        const QString subject = lines[i + 3];

        // Three lines: hash, subject, author · date
        const QString display = hash + "\n" + subject + "\n" + author + "  \u00b7  " + date;

        auto *item = new QListWidgetItem(display);
        item->setData(Qt::UserRole, hash);
        m_commitHistoryList->addItem(item);
    }
}

void MainWindow::onHistoryItemClicked(QListWidgetItem *item)
{
    if (!item)
        return;

    m_selectedCommitHash = item->data(Qt::UserRole).toString();
    if (m_selectedCommitHash.isEmpty())
        return;

    m_fileContentViewer->clear();
    m_viewerStack->setCurrentIndex(1);

    m_commitFilesList->clear();
    m_commitFilesList->setVisible(true);
    if (m_commitFilesHeader)
        m_commitFilesHeader->setVisible(true);
    if (m_commitFilesContainer)
        m_commitFilesContainer->setVisible(true);
    if (m_viewCommitFilesAction)
        m_viewCommitFilesAction->setChecked(true);

    m_commitDetailProcess->setWorkingDirectory(m_repoPath);
    m_commitDetailProcess->start("git", {
        "diff-tree", "--no-commit-id", "-r", "--name-status",
        m_selectedCommitHash
    });
}

void MainWindow::onCommitDetailFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit || exitCode != 0)
        return;

    m_commitFilesList->clear();

    const QString output = QString::fromUtf8(
        m_commitDetailProcess->readAllStandardOutput());
    const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

    for (const QString &line : lines) {
        // Format: "M\tpath/to/file" or "A\tpath/to/file" etc.
        const int tabPos = line.indexOf('\t');
        if (tabPos < 0)
            continue;

        const QString status = line.left(tabPos);
        const QString filePath = line.mid(tabPos + 1);
        const QString fileName = filePath.section('/', -1);

        // Show "M  filename.ext" with status prefix
        auto *item = new QListWidgetItem(status + "  " + fileName);
        item->setData(Qt::UserRole, filePath);

        // Color the status letter
        QColor color;
        if (status == "M")       color = QColor("#f0c000");
        else if (status == "A")  color = QColor("#40c040");
        else if (status == "D")  color = QColor("#e04040");
        else if (status == "R")  color = QColor("#c080ff");
        else                     color = QColor("#c0c0c0");

        item->setForeground(color);
        m_commitFilesList->addItem(item);
    }
}

void MainWindow::onCommitFileClicked(QListWidgetItem *item)
{
    if (!item)
        return;

    const QString filePath = item->data(Qt::UserRole).toString();
    if (filePath.isEmpty() || m_selectedCommitHash.isEmpty())
        return;

    auto *diffProc = new QProcess(this);
    diffProc->setWorkingDirectory(m_repoPath);
    diffProc->start("git", {"show", m_selectedCommitHash, "--", filePath});

    m_viewerStack->setCurrentIndex(0);
    connect(diffProc, &QProcess::finished, this, [this, diffProc](int ec, QProcess::ExitStatus es) {
        diffProc->deleteLater();
        if (es != QProcess::NormalExit || ec != 0)
            return;

        m_fileContentViewer->setDiff(
            QString::fromUtf8(diffProc->readAllStandardOutput()));
    });
}

// --- File system watcher (auto-refresh) ---

void MainWindow::onRepoDirChanged()
{
    m_refreshTimer->start();
}

void MainWindow::onRefreshDebounce()
{
    if (m_repoPath.isEmpty())
        return;
    m_currentQuery = GitQuery::None;
    startGitStatusQuery();
}

// --- Git bootstrapping ---

bool MainWindow::checkGitAvailable()
{
    const QString gitPath = QStandardPaths::findExecutable("git");
    if (!gitPath.isEmpty())
        return true;

    auto reply = QMessageBox::question(this, "Git Not Found",
        "Git is required but was not found on your system.\n\n"
        "Would you like to install it now?",
        QMessageBox::Yes | QMessageBox::No);

    if (reply == QMessageBox::Yes) {
        installGit();
    } else {
        QMessageBox::information(this, "Git Required",
            "Some features will be unavailable without Git.\n"
            "You can install it manually and restart the application.");
    }

    return false;
}

void MainWindow::installGit()
{
    if (m_installProcess->state() == QProcess::Running)
        return;

    QStringList args;

#ifdef Q_OS_WIN
    args = {"install", "--id", "Git.Git", "-e", "--source", "winget"};
    m_installProcess->start("winget", args);
#elif defined(Q_OS_MACOS)
    args = {"--install"};
    m_installProcess->start("xcode-select", args);
#else
    // Linux — try apt-get, then dnf, then pacman
    const QString pm = QStandardPaths::findExecutable("apt-get").isEmpty()
                           ? (QStandardPaths::findExecutable("dnf").isEmpty()
                                  ? "pacman"
                                  : "dnf")
                           : "apt-get";

    if (pm == "apt-get")
        args = {"install", "-y", "git"};
    else if (pm == "dnf")
        args = {"install", "-y", "git"};
    else
        args = {"-S", "--noconfirm", "git"};

    QString runner = QStandardPaths::findExecutable("pkexec");
    if (runner.isEmpty())
        runner = QStandardPaths::findExecutable("sudo");

    if (runner.isEmpty()) {
        QMessageBox::critical(this, "Installation Failed",
            "Could not find a privilege escalation tool (pkexec or sudo).\n"
            "Please install Git manually.");
        return;
    }

    m_installProcess->start(runner, QStringList({pm}) + args);
#endif
}

void MainWindow::onInstallFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit)
        return;

    if (exitCode != 0) {
        const QString err = QString::fromUtf8(
            m_installProcess->readAllStandardError());
        QMessageBox::warning(this, "Installation Failed", err);
        return;
    }

    if (checkGitAvailable()) {
        QMessageBox::information(this, "Installation Complete",
            "Git has been installed successfully.");
    }
}

// --- Git authentication ---

void MainWindow::checkGitAuth()
{
    if (!m_gitCredentials.valid)
        return;

    m_authProcess->setWorkingDirectory(m_repoPath);

    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    env.insert("GIT_TERMINAL_PROMPT", "0");
    m_authProcess->setProcessEnvironment(env);

    m_authProcess->start("git", {"ls-remote", "--exit-code",
        m_gitCredentials.host});
}

void MainWindow::onAuthCheckFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit)
        return;

    // exit code 128 usually means auth failure
    if (exitCode == 0 || exitCode == 2)
        return;

    showAuthDialog();
}

void MainWindow::showAuthDialog()
{
    QDialog dialog(this);
    dialog.setWindowTitle("Git Authentication");
    dialog.setMinimumWidth(400);

    auto *form = new QFormLayout(&dialog);

    auto *hostEdit = new QLineEdit();
    hostEdit->setPlaceholderText("e.g. https://github.com");

    auto *userEdit = new QLineEdit();
    userEdit->setPlaceholderText("Username");

    auto *tokenEdit = new QLineEdit();
    tokenEdit->setPlaceholderText("Personal Access Token or Password");
    tokenEdit->setEchoMode(QLineEdit::Password);

    auto *buttons = new QDialogButtonBox(
        QDialogButtonBox::Ok | QDialogButtonBox::Cancel);

    form->addRow("Remote Host:", hostEdit);
    form->addRow("Username:", userEdit);
    form->addRow("Token / Password:", tokenEdit);
    form->addRow(buttons);

    connect(buttons, &QDialogButtonBox::accepted, &dialog, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, &dialog, &QDialog::reject);

    if (dialog.exec() != QDialog::Accepted)
        return;

    m_gitCredentials.host = hostEdit->text().trimmed();
    m_gitCredentials.username = userEdit->text().trimmed();
    m_gitCredentials.token = tokenEdit->text();
    m_gitCredentials.valid = true;

    setupAskPass();
}

bool MainWindow::setupAskPass()
{
    cleanupAskPass();

    QTemporaryFile tmp;
    tmp.setAutoRemove(false);
    if (!tmp.open())
        return false;
    m_askPassScriptPath = tmp.fileName();
    tmp.close();

    QFile file(m_askPassScriptPath);

#ifdef Q_OS_WIN
    if (!file.open(QIODevice::WriteOnly))
        return false;
    QTextStream out(&file);
    out << "@echo off\n";
    out << "if \"%1\"==\"*Username*\" ( echo " << m_gitCredentials.username << " ) else ( echo " << m_gitCredentials.token << " )\n";
    file.close();
#else
    if (!file.open(QIODevice::WriteOnly))
        return false;
    QTextStream out(&file);
    out << "#!/bin/sh\n";
    out << "case \"$1\" in\n";
    out << "  *Username*) echo \"" << m_gitCredentials.username << "\" ;;\n";
    out << "  *)           echo \"" << m_gitCredentials.token << "\" ;;\n";
    out << "esac\n";
    file.close();
    file.setPermissions(QFile::ExeOwner | QFile::ExeGroup | QFile::ExeOther |
                        QFile::ReadOwner | QFile::WriteOwner);
#endif

    return true;
}

void MainWindow::cleanupAskPass()
{
    if (!m_askPassScriptPath.isEmpty()) {
        QFile::remove(m_askPassScriptPath);
        m_askPassScriptPath.clear();
    }
}

void MainWindow::setupAuthEnv(QProcess *proc)
{
    if (!m_gitCredentials.valid || m_askPassScriptPath.isEmpty())
        return;

    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    env.insert("GIT_ASKPASS", m_askPassScriptPath);
    env.insert("GIT_TERMINAL_PROMPT", "0");
    proc->setProcessEnvironment(env);
}
