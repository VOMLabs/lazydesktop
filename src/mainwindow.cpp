#include "mainwindow.h"

#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFont>
#include <QFontDatabase>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPushButton>
#include <QRegularExpression>
#include <QSettings>
#include <QSplitter>
#include <QTextCursor>
#include <QTextEdit>
#include <QTreeWidget>
#include <QTreeWidgetItem>
#include <QVBoxLayout>

static const int kMaxRecentProjects = 10;

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setupUi();
    loadRecentProjects();
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

    topLayout->addWidget(m_projectButton);
    topLayout->addWidget(m_currentPathLabel);
    topLayout->addStretch();
    topLayout->addWidget(m_pushButton);

    // --- Recent projects drawer (hidden by default) ---
    m_recentDrawer = new QWidget();
    m_recentDrawer->setFixedWidth(260);
    m_recentDrawer->setVisible(false);

    auto *drawerLayout = new QVBoxLayout(m_recentDrawer);
    drawerLayout->setContentsMargins(6, 6, 6, 6);

    m_openProjectButton = new QPushButton("+ Open Project");
    m_openProjectButton->setMinimumHeight(32);

    m_recentList = new QListWidget();
    m_recentList->setAlternatingRowColors(true);

    drawerLayout->addWidget(m_openProjectButton);
    drawerLayout->addWidget(m_recentList, 1);

    // --- Main content widgets ---
    m_gitStatusTree = new QTreeWidget();
    m_gitStatusTree->setHeaderHidden(true);
    m_gitStatusTree->setColumnCount(1);

    m_fileContentViewer = new QPlainTextEdit();
    m_fileContentViewer->setReadOnly(true);
    auto monoFont = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    m_fileContentViewer->setFont(monoFont);

    // --- Commit pane ---
    m_summaryInput = new QLineEdit();
    m_summaryInput->setPlaceholderText("Summary (required)");

    m_descriptionInput = new QTextEdit();
    m_descriptionInput->setPlaceholderText("Description (optional)");
    m_descriptionInput->setFixedHeight(80);
    m_descriptionInput->setAcceptRichText(false);

    m_commitButton = new QPushButton("Commit");
    m_commitButton->setEnabled(false);

    auto *commitLayout = new QVBoxLayout();
    commitLayout->setContentsMargins(4, 4, 4, 4);
    commitLayout->addWidget(m_summaryInput);
    commitLayout->addWidget(m_descriptionInput);
    commitLayout->addWidget(m_commitButton);

    auto *commitContainer = new QWidget();
    commitContainer->setLayout(commitLayout);

    // Left panel: tree on top, commit pane on bottom
    auto *leftPanel = new QSplitter(Qt::Vertical);
    leftPanel->addWidget(m_gitStatusTree);
    leftPanel->addWidget(commitContainer);
    leftPanel->setStretchFactor(0, 1);
    leftPanel->setStretchFactor(1, 0);

    // Main horizontal splitter
    auto *splitter = new QSplitter(Qt::Horizontal);
    splitter->addWidget(leftPanel);
    splitter->addWidget(m_fileContentViewer);
    splitter->setStretchFactor(0, 1);
    splitter->setStretchFactor(1, 2);

    // Content area: drawer + main splitter
    auto *contentLayout = new QHBoxLayout();
    contentLayout->setContentsMargins(0, 0, 0, 0);
    contentLayout->setSpacing(0);
    contentLayout->addWidget(m_recentDrawer);
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

    m_pushProcess = new QProcess(this);
    connect(m_pushProcess, &QProcess::finished,
            this, &MainWindow::onPushFinished);
    connect(m_pushProcess, &QProcess::errorOccurred,
            this, &MainWindow::onPushErrorOccurred);

    // Connections
    connect(m_projectButton, &QPushButton::clicked,
            this, &MainWindow::onProjectButtonClicked);
    connect(m_openProjectButton, &QPushButton::clicked,
            this, &MainWindow::onProjectButtonClicked);
    connect(m_recentList, &QListWidget::itemClicked,
            this, &MainWindow::onRecentProjectClicked);
    connect(m_gitStatusTree, &QTreeWidget::itemClicked,
            this, &MainWindow::onTreeItemClicked);
    connect(m_summaryInput, &QLineEdit::textChanged,
            this, &MainWindow::onSummaryTextChanged);
    connect(m_commitButton, &QPushButton::clicked,
            this, &MainWindow::onCommitClicked);
    connect(m_pushButton, &QPushButton::clicked,
            this, &MainWindow::onPushClicked);
}

// --- Recent projects persistence ---

void MainWindow::loadRecentProjects()
{
    QSettings settings;
    m_recentProjects = settings.value("recentProjects").toStringList();
    populateRecentList();
}

void MainWindow::saveRecentProjects()
{
    QSettings settings;
    settings.setValue("recentProjects", m_recentProjects);
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
    for (const QString &path : m_recentProjects) {
        auto *item = new QListWidgetItem(QDir(path).dirName());
        item->setData(Qt::UserRole, path);
        item->setToolTip(path);
        m_recentList->addItem(item);
    }
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
        // Project open → toggle drawer
        m_recentDrawer->setVisible(!m_recentDrawer->isVisible());
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
    m_treeDirs.clear();
    m_fileContentViewer->clear();
    m_recentDrawer->setVisible(false);

    addRecentProject(path);
    startGitStatusQuery();
    return true;
}

bool MainWindow::isGitRepository(const QString &path)
{
    const QFileInfo gitInfo(QDir(path).filePath(".git"));
    return gitInfo.exists();
}

void MainWindow::addGitFileToTree(const QString &path, const QString &prefix)
{
    const QStringList parts = path.split('/');

    QTreeWidgetItem *parent = nullptr;
    QString accumulated;

    for (int i = 0; i < parts.size() - 1; ++i) {
        if (!accumulated.isEmpty())
            accumulated += '/';
        accumulated += parts[i];

        auto it = m_treeDirs.constFind(accumulated);
        if (it != m_treeDirs.constEnd()) {
            parent = it.value();
        } else {
            auto *dirItem = new QTreeWidgetItem();
            dirItem->setText(0, parts[i] + '/');
            dirItem->setFlags(dirItem->flags() & ~Qt::ItemIsSelectable);
            if (parent)
                parent->addChild(dirItem);
            else
                m_gitStatusTree->addTopLevelItem(dirItem);
            m_treeDirs[accumulated] = dirItem;
            parent = dirItem;
        }
    }

    auto *fileItem = new QTreeWidgetItem();
    fileItem->setText(0, QString("%1 %2").arg(prefix, parts.last()));
    fileItem->setData(0, Qt::UserRole, path);
    if (parent)
        parent->addChild(fileItem);
    else
        m_gitStatusTree->addTopLevelItem(fileItem);
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

    if (!item || item->text(0).endsWith('/'))
        return;

    const QString relPath = item->data(0, Qt::UserRole).toString();
    if (relPath.isEmpty())
        return;

    m_fileContentViewer->clear();

    QString diff = runGitDiff(m_repoPath, {"diff", "HEAD", "--", relPath});

    if (diff.isEmpty())
        diff = runGitDiff(m_repoPath, {"diff", "@{u}..HEAD", "--", relPath});

    if (diff.isEmpty()) {
        m_fileContentViewer->setPlainText("No changes to display.");
        return;
    }

    auto *doc = m_fileContentViewer->document();
    QTextCursor cursor(doc);
    QRegularExpression hunkRe(R"(@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@.*)");

    const QStringList lines = diff.split('\n');
    int oldLn = 0;
    int newLn = 0;

    for (const QString &line : lines) {
        if (line.startsWith("---") || line.startsWith("+++")
            || line.startsWith("diff --git")
            || line.startsWith("\\ "))
            continue;

        QTextCharFormat fmt;
        QString display;

        auto match = hunkRe.match(line);
        if (match.hasMatch()) {
            oldLn = match.captured(1).toInt();
            newLn = match.captured(2).toInt();
            fmt.setForeground(QColor(80, 80, 200));
            fmt.setFontWeight(QFont::Bold);
            display = line;
        } else if (line.startsWith('-')) {
            fmt.setForeground(Qt::black);
            fmt.setBackground(QColor(255, 200, 200));
            display = QString("%1%2").arg(oldLn).arg(line);
            oldLn++;
        } else if (line.startsWith('+')) {
            fmt.setForeground(Qt::black);
            fmt.setBackground(QColor(200, 255, 200));
            display = QString("%1%2").arg(newLn).arg(line);
            newLn++;
        } else if (line.startsWith(' ')) {
            oldLn++;
            newLn++;
            continue;
        } else {
            continue;
        }

        cursor.insertText(display + '\n', fmt);
    }
}

// --- Commit pane ---

void MainWindow::onSummaryTextChanged(const QString &text)
{
    m_commitButton->setEnabled(!text.trimmed().isEmpty());
}

void MainWindow::onCommitClicked()
{
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

    QStringList args = {"commit", "-a", "-m", m_summaryInput->text().trimmed()};

    const QString desc = m_descriptionInput->toPlainText().trimmed();
    if (!desc.isEmpty())
        args << "-m" << desc;

    m_commitButton->setEnabled(false);
    m_commitProcess->setWorkingDirectory(m_repoPath);
    m_commitProcess->start("git", args);
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

    m_gitStatusTree->clear();
    m_treeDirs.clear();
    m_fileContentViewer->clear();
    startGitStatusQuery();
}

void MainWindow::onCommitErrorOccurred(QProcess::ProcessError error)
{
    if (error == QProcess::FailedToStart) {
        QMessageBox::critical(this, "Git Not Found",
            "Git is not installed or not available on the system PATH.");
    }

    m_commitProcess->deleteLater();
    m_commitProcess = nullptr;
    m_commitButton->setEnabled(!m_summaryInput->text().trimmed().isEmpty());
}

// --- Push / Fetch / Pull ---

void MainWindow::onPushClicked()
{
    if (m_pushProcess->state() != QProcess::NotRunning)
        return;

    m_pushButton->setEnabled(false);
    m_pushProcess->setWorkingDirectory(m_repoPath);

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
            m_treeDirs.clear();
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
        m_treeDirs.clear();
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
    if (m_gitProcess->state() != QProcess::NotRunning) {
        m_gitProcess->kill();
        m_gitProcess->waitForFinished(500);
    }

    m_currentQuery = GitQuery::Status;
    m_gitProcess->setWorkingDirectory(m_repoPath);
    m_gitProcess->start("git", {"status", "--porcelain"});
}

void MainWindow::startGitUnpushedQuery()
{
    if (m_gitProcess->state() != QProcess::NotRunning) {
        m_gitProcess->kill();
        m_gitProcess->waitForFinished(500);
    }

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

        m_gitStatusTree->expandAll();
        startGitUnpushedQuery();
    } else if (m_currentQuery == GitQuery::Unpushed) {
        const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

        for (const QString &line : lines) {
            const QString trimmed = line.trimmed();
            if (trimmed.isEmpty())
                continue;

            addGitFileToTree(trimmed, "[P]");
        }

        m_gitStatusTree->expandAll();
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
