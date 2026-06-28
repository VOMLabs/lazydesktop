#include "mainwindow.h"

#include <QDir>
#include <QFile>
#include <QFileDialog>
#include <QFileInfo>
#include <QFont>
#include <QFontDatabase>
#include <QHBoxLayout>
#include <QLabel>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProcess>
#include <QPushButton>
#include <QSplitter>
#include <QTreeWidget>
#include <QTreeWidgetItem>
#include <QVBoxLayout>

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setupUi();
}

MainWindow::~MainWindow()
{
    if (m_gitProcess && m_gitProcess->state() != QProcess::NotRunning) {
        m_gitProcess->kill();
        m_gitProcess->waitForFinished(3000);
    }
}

void MainWindow::setupUi()
{
    setWindowTitle("lazydesktop");
    resize(1000, 600);

    auto *centralWidget = new QWidget(this);
    auto *mainLayout = new QVBoxLayout(centralWidget);

    auto *topLayout = new QHBoxLayout();

    m_openFolderButton = new QPushButton("Open Folder");
    m_currentPathLabel = new QLabel("No folder selected");
    m_gitStatusTree = new QTreeWidget();
    m_fileContentViewer = new QPlainTextEdit();

    m_gitStatusTree->setHeaderHidden(true);
    m_gitStatusTree->setColumnCount(1);

    m_fileContentViewer->setReadOnly(true);
    auto monoFont = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    m_fileContentViewer->setFont(monoFont);

    auto *splitter = new QSplitter(Qt::Horizontal);
    splitter->addWidget(m_gitStatusTree);
    splitter->addWidget(m_fileContentViewer);
    splitter->setStretchFactor(0, 1);
    splitter->setStretchFactor(1, 2);

    topLayout->addWidget(m_openFolderButton);
    topLayout->addWidget(m_currentPathLabel);
    topLayout->addStretch();

    mainLayout->addLayout(topLayout);
    mainLayout->addWidget(splitter, 1);

    setCentralWidget(centralWidget);

    m_gitProcess = new QProcess(this);
    connect(m_gitProcess, &QProcess::finished,
            this, &MainWindow::onGitProcessFinished);
    connect(m_gitProcess, &QProcess::errorOccurred,
            this, &MainWindow::onGitProcessErrorOccurred);

    connect(m_openFolderButton, &QPushButton::clicked,
            this, &MainWindow::onOpenFolder);
    connect(m_gitStatusTree, &QTreeWidget::itemClicked,
            this, &MainWindow::onTreeItemClicked);
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

void MainWindow::onOpenFolder()
{
    const QString dir = QFileDialog::getExistingDirectory(
        this, "Open Git Repository");

    if (dir.isEmpty())
        return;

    if (!isGitRepository(dir)) {
        QMessageBox::warning(this, "Invalid Folder",
            "The selected folder is not a Git repository.\n"
            "Please select a folder that contains a .git directory.");
        return;
    }

    m_repoPath = dir;
    m_currentPathLabel->setText(dir);
    m_gitStatusTree->clear();
    m_treeDirs.clear();
    m_fileContentViewer->clear();

    startGitStatusQuery();
}

void MainWindow::onTreeItemClicked(QTreeWidgetItem *item, int column)
{
    Q_UNUSED(column);

    if (!item || item->text(0).endsWith('/'))
        return;

    const QString relPath = item->data(0, Qt::UserRole).toString();
    if (relPath.isEmpty())
        return;

    QFile file(m_repoPath + '/' + relPath);
    if (!file.open(QIODevice::ReadOnly)) {
        m_fileContentViewer->setPlainText(
            QString("Error opening file: %1").arg(file.errorString()));
        return;
    }

    m_fileContentViewer->setPlainText(QString::fromUtf8(file.readAll()));
}

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
