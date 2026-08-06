#include "diffviewer.h"
#include "mainwindow.h"
#include "model_manager_bridge.h"

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
#include <QProgressBar>
#include <QPushButton>
#include <QRegularExpression>
#include <QScrollArea>
#include <QSet>
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
#include <QApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QUrl>
#include <QVBoxLayout>
#include <memory>

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

class FileTreeDelegate : public QStyledItemDelegate {
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

        if (selected)
            painter->fillRect(opt.rect, opt.palette.highlight());
        else if (hovered)
            painter->fillRect(opt.rect, opt.palette.alternateBase());

        QRect r = opt.rect;

        // 1. Draw colored status square
        QVariant iconVar = index.data(Qt::DecorationRole);
        if (iconVar.canConvert<QIcon>()) {
            QIcon icon = iconVar.value<QIcon>();
            QPixmap pm = icon.pixmap(8, 8);
            int iconX = r.left() + 4;
            int iconY = r.top() + (r.height() - 8) / 2;
            painter->drawPixmap(iconX, iconY, pm);
        }

        // 2. Draw checkbox
        QStyleOptionViewItem opt2 = opt;
        opt2.rect = QRect(r.left() + 18, r.top(), 20, r.height());
        QStyle *style = opt.widget ? opt.widget->style() : QApplication::style();
        style->drawPrimitive(QStyle::PE_IndicatorItemViewItemCheck, &opt2, painter, opt.widget);

        // 3. Draw text
        QRect textRect = r.adjusted(42, 0, -4, 0);
        QString text = index.data(Qt::DisplayRole).toString();
        painter->setPen(selected ? opt.palette.highlightedText().color()
                                 : opt.palette.windowText().color());
        painter->drawText(textRect, Qt::AlignLeft | Qt::AlignVCenter | Qt::TextSingleLine, text);

        painter->restore();
    }

    QSize sizeHint(const QStyleOptionViewItem &option,
                   const QModelIndex &) const override
    {
        return QSize(200, option.fontMetrics.height() + 8);
    }
};

static const int kMaxRecentProjects = 100;

static QString defaultSettingsDir()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation);
}

static QString defaultProjectsPath()
{
    return defaultSettingsDir() + "/projects.yaml";
}

static QString defaultSettingsPath()
{
    return defaultSettingsDir() + "/lazydesktop.conf";
}

static QString defaultThemesDir()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppConfigLocation)
           + "/themes";
}

static std::unique_ptr<QSettings> lazySettings()
{
    const QString path = defaultSettingsPath();
    QDir().mkpath(defaultSettingsDir());
    return std::make_unique<QSettings>(path, QSettings::IniFormat);
}

static QString projectsFilePath()
{
    auto s = lazySettings();
    return s->value("paths/projects", defaultProjectsPath()).toString();
}

static QString settingsFilePath()
{
    auto s = lazySettings();
    return s->value("paths/settings", defaultSettingsPath()).toString();
}

static QString themesDirPath()
{
    auto s = lazySettings();
    return s->value("paths/themes", defaultThemesDir()).toString();
}

static QString defaultModelsDir()
{
    return QStandardPaths::writableLocation(QStandardPaths::AppDataLocation)
           + "/models";
}

static QString modelsDirPath()
{
    return defaultModelsDir();
}

static const QString kDefaultDescriptionSystemPrompt = QStringLiteral(
    "Write a casual, plain-language description of the changes below. "
    "Explain what changed and why in a few sentences. Do not include a summary title line.\n"
    "\n"
    "Diff:\n"
    "<diff>\n"
    "\n"
    "Description:");

static QString detectVcsKind(const QString &repoPath)
{
    if (!repoPath.isEmpty() && QFileInfo::exists(repoPath + QLatin1String("/.jj")))
        return QStringLiteral("jujutsu");
    return QStringLiteral("git");
}

static QString runVcsCommand(const QString &repoPath, const QString &program,
                             const QStringList &args)
{
    QProcess p;
    p.setWorkingDirectory(repoPath);
    p.start(program, args);
    if (p.waitForFinished(3000) && p.exitCode() == 0)
        return QString::fromUtf8(p.readAllStandardOutput());
    return {};
}

static QString buildDiffText(const QString &repoPath, const QStringList &files)
{
    const QString vcs = detectVcsKind(repoPath);
    QStringList diffParts;
    for (const QString &file : files) {
        QString d;
        if (vcs == QLatin1String("jujutsu"))
            d = runVcsCommand(repoPath, "jj", {"diff", "--color", "never", "--", file}).trimmed();
        else
            d = runVcsCommand(repoPath, "git", {"diff", "HEAD", "--", file}).trimmed();
        if (!d.isEmpty())
            diffParts << "--- " + file + "\n" + d;
    }
    return diffParts.join("\n\n");
}

static QJsonArray fileStatusesJson(const QString &repoPath, const QString &vcs,
                                   const QStringList &files)
{
    QJsonArray out;
    if (files.isEmpty())
        return out;

    QHash<QString, QString> statusByPath;
    if (vcs == QLatin1String("jujutsu")) {
        const QString outTxt = runVcsCommand(repoPath, "jj", {"status"});
        bool inSection = false;
        for (const QString &line : outTxt.split(QLatin1Char('\n'))) {
            if (line.startsWith(QLatin1String("Working copy changes"))) {
                inSection = true;
                continue;
            }
            if (line.trimmed().isEmpty() || line.endsWith(QLatin1Char(':'))) {
                inSection = false;
                continue;
            }
            if (inSection && line.startsWith(QLatin1Char(' '))) {
                const QString trimmed = line.trimmed();
                if (trimmed.length() >= 2)
                    statusByPath.insert(trimmed.mid(1).trimmed(), trimmed.left(1));
            }
        }
    } else {
        const QString outTxt = runVcsCommand(repoPath, "git", {"status", "--porcelain"});
        for (const QString &line : outTxt.split(QLatin1Char('\n'))) {
            if (line.length() < 4)
                continue;
            const QChar status = line.at(0) != QLatin1Char(' ') ? line.at(0) : line.at(1);
            QString path = line.mid(3).trimmed();
            if (path.contains(QLatin1String(" -> ")))
                path = path.section(QLatin1String(" -> "), -1, -1);
            statusByPath.insert(path, QString(status));
        }
    }

    for (const QString &file : files) {
        QJsonObject o;
        o["path"] = file;
        o["status"] = statusByPath.value(file, QStringLiteral("M"));
        out.append(o);
    }
    return out;
}

static QStringList stagedFiles(const QString &repoPath, const QString &vcs,
                               const QStringList &checked)
{
    if (vcs == QLatin1String("jujutsu"))
        return checked;
    const QString outTxt = runVcsCommand(repoPath, "git", {"diff", "--cached", "--name-only"});
    if (outTxt.trimmed().isEmpty())
        return {};
    return outTxt.trimmed().split(QLatin1Char('\n'));
}

static QString currentBranchOrChange(const QString &repoPath, const QString &vcs)
{
    if (vcs == QLatin1String("jujutsu")) {
        return runVcsCommand(
                   repoPath, "jj",
                   {"log", "--no-graph", "-r", "@", "--limit", "1",
                    "--config", "ui.pagination=never", "-T", "change_id.short()"})
            .trimmed();
    }
    QString branch = runVcsCommand(repoPath, "git", {"symbolic-ref", "--short", "HEAD"}).trimmed();
    if (branch.isEmpty())
        branch = runVcsCommand(repoPath, "git", {"rev-parse", "--short", "HEAD"}).trimmed();
    return branch;
}

static QStringList recentCommitMessages(const QString &repoPath, const QString &vcs)
{
    QString outTxt;
    if (vcs == QLatin1String("jujutsu")) {
        outTxt = runVcsCommand(
            repoPath, "jj",
            {"log", "--no-graph", "--limit", "10", "--config", "ui.pagination=never",
             "-T", "change_id.short() ++ \" \" ++ description.first_line()"});
    } else {
        outTxt = runVcsCommand(repoPath, "git", {"log", "--oneline", "-10"});
    }
    if (outTxt.trimmed().isEmpty())
        return {};
    return outTxt.trimmed().split(QLatin1Char('\n'));
}

// If rawPrompt contains the "<diff>" placeholder, substitutes the actual diff there and
// leaves the user message minimal. Otherwise falls back to appending the diff as the
// user message, preserving behavior for prompts written before the placeholder existed.
struct AiPrompt {
    QString systemPrompt;
    QString userContent;
};

static AiPrompt buildAiPrompt(const QString &rawPrompt, const QString &diffText)
{
    AiPrompt result;
    if (rawPrompt.contains(QLatin1String("<diff>"))) {
        result.systemPrompt = QString(rawPrompt).replace(QLatin1String("<diff>"), diffText);
        result.userContent = QStringLiteral("Generate the response now.");
    } else {
        result.systemPrompt = rawPrompt;
        result.userContent = "Changes:\n" + diffText;
    }
    return result;
}

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent)
{
    setupUi();

    m_modelManager = new ModelManagerBridge(
        modelsDirPath(), defaultSettingsDir(), this);
    connect(m_modelManager, &ModelManagerBridge::inferenceFinished,
            this, &MainWindow::onLocalAiResponse);
    connect(m_modelManager, &ModelManagerBridge::inferenceError,
            this, &MainWindow::onLocalAiError);
    connect(m_modelManager, &ModelManagerBridge::inferenceToken,
            this, &MainWindow::onLocalAiThinking);

    loadRecentProjects();
    checkGitAvailable();
    applySavedTheme();
}

MainWindow::~MainWindow()
{
    if (m_modelManager) {
        m_modelManager->cancelInference();
    }
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

    m_deleteBranchButton = new QPushButton();
    m_deleteBranchButton->setFixedSize(24, 24);
    m_deleteBranchButton->setEnabled(false);
    m_deleteBranchButton->setIcon(style()->standardIcon(QStyle::SP_DialogCloseButton));
    m_deleteBranchButton->setIconSize(QSize(14, 14));
    m_deleteBranchButton->setStyleSheet(
        "QPushButton { border: none; }"
        "QPushButton:hover { background: #ffcccc; border-radius: 3px; }");

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

    m_addProjectButton = new QPushButton("+ Add");
    m_addProjectButton->setMinimumHeight(32);
    {
        auto *menu = new QMenu(m_addProjectButton);
        menu->addAction("Clone Repository", this, &MainWindow::onCloneRepository);
        menu->addAction("Create Repository", this, &MainWindow::onCreateRepository);
        menu->addAction("Load Existing", this, &MainWindow::onOpenExistingProject);
        m_addProjectButton->setMenu(menu);
    }

    m_recentList = new QListWidget();
    m_recentList->setAlternatingRowColors(true);

    frameLayout->addWidget(m_addProjectButton);

    auto *scanButton = new QPushButton("Scan Folder for Projects");
    scanButton->setMinimumHeight(28);
    scanButton->setToolTip("Scan a folder for subdirectories that are Git repositories and add them all");
    connect(scanButton, &QPushButton::clicked, this, &MainWindow::onScanFolder);
    frameLayout->addWidget(scanButton);

    frameLayout->addWidget(m_recentList, 1);

    auto *clearAllBtn = new QPushButton("Remove All");
    clearAllBtn->setFixedHeight(24);
    clearAllBtn->setStyleSheet(
        "QPushButton { color: #888; font-size: 11px; border: none; }"
        "QPushButton:hover { color: #e04040; }");
    connect(clearAllBtn, &QPushButton::clicked, this, &MainWindow::onClearAllProjects);
    frameLayout->addWidget(clearAllBtn);

    // --- Header bar with Select All ---
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
    m_selectAllCheck->setTristate(false);
    auto *headerLabel = new QLabel("Changed Files");
    {
        QFont bf = headerLabel->font();
        bf.setBold(true);
        headerLabel->setFont(bf);
    }
    m_changedFilesLabel = new QLabel("0 files");
    {
        QFont bf = m_changedFilesLabel->font();
        bf.setBold(true);
        m_changedFilesLabel->setFont(bf);
        QPalette lp = m_changedFilesLabel->palette();
        lp.setColor(QPalette::WindowText, QColor("#f0c000"));
        m_changedFilesLabel->setPalette(lp);
    }
    headerLayout->addWidget(m_selectAllCheck);
    headerLayout->addWidget(headerLabel);
    headerLayout->addStretch();
    headerLayout->addWidget(m_changedFilesLabel);

    connect(m_selectAllCheck, &QCheckBox::checkStateChanged, this, &MainWindow::setAllCheckStates);

    // --- Git status tree (single file tree with checkboxes) ---
    m_gitStatusTree = new QTreeWidget();
    m_gitStatusTree->setHeaderHidden(true);
    m_gitStatusTree->setColumnCount(1);
    m_gitStatusTree->setContextMenuPolicy(Qt::CustomContextMenu);
    m_gitStatusTree->setRootIsDecorated(false);
    m_gitStatusTree->setIndentation(0);
    m_gitStatusTree->setAnimated(false);
    m_gitStatusTree->setIconSize(QSize(10, 10));
    m_gitStatusTree->setItemDelegate(new FileTreeDelegate(m_gitStatusTree));
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

    m_binaryPreview = new QLabel();
    m_binaryPreview->setAlignment(Qt::AlignCenter);
    m_binaryPreview->setScaledContents(false);
    m_binaryPreview->setStyleSheet("background: #1e1e1e;");
    m_binaryPreview->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);

    m_viewerStack = new QStackedWidget();
    m_viewerStack->addWidget(m_fileContentViewer);  // page 0 – diff
    m_viewerStack->addWidget(m_placeholderWidget);  // page 1 – empty
    m_viewerStack->addWidget(m_binaryPreview);      // page 2 – image/video
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
    auto *commitCloseBtn = new QPushButton();
    commitCloseBtn->setFixedSize(20, 20);
    commitCloseBtn->setFlat(true);
    commitCloseBtn->setIcon(style()->standardIcon(QStyle::SP_DialogCloseButton));
    commitCloseBtn->setIconSize(QSize(12, 12));
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

    m_aiCommitButton = new QPushButton();
    m_aiCommitButton->setFixedSize(28, 28);
    m_aiCommitButton->setEnabled(false);
    m_aiCommitButton->setFlat(true);
    m_aiCommitButton->setToolTip("Generate commit message with AI");
    {
        QPixmap pm(20, 20);
        pm.fill(Qt::transparent);
        QPainter p(&pm);
        p.setPen(QColor("#d4d4d4"));
        QFont f = p.font();
        f.setPixelSize(13);
        f.setBold(true);
        p.setFont(f);
        p.drawText(QRect(0, 0, 20, 20), Qt::AlignCenter, "AI");
        p.end();
        m_aiCommitButton->setIcon(QIcon(pm));
        m_aiCommitButton->setIconSize(QSize(20, 20));
    }
    connect(m_aiCommitButton, &QPushButton::clicked, this, &MainWindow::onGenerateCommitMessage);

    m_aiCommitButton->setContextMenuPolicy(Qt::CustomContextMenu);
    connect(m_aiCommitButton, &QPushButton::customContextMenuRequested, this, [this](const QPoint &pos) {
        QMenu menu;
        QStringList providers{"OpenRouter", "OpenAI", "Anthropic",
                          "Google AI Studio", "Local (internal llama.cpp)"};
        auto settings = lazySettings();
        QString current = settings->value("ai/provider", "OpenRouter").toString();
        for (const QString &p : providers) {
            auto *action = menu.addAction(p, this, [p]() {
                auto s = lazySettings();
                s->setValue("ai/provider", p);
            });
            if (p == current)
                action->setCheckable(true);
        }
        menu.exec(m_aiCommitButton->mapToGlobal(pos));
    });

    m_aiDescriptionButton = new QPushButton();
    m_aiDescriptionButton->setFixedSize(28, 28);
    m_aiDescriptionButton->setEnabled(false);
    m_aiDescriptionButton->setFlat(true);
    m_aiDescriptionButton->setToolTip("Generate commit description with AI");
    {
        QPixmap pm(20, 20);
        pm.fill(Qt::transparent);
        QPainter p(&pm);
        p.setPen(QColor("#d4d4d4"));
        QFont f = p.font();
        f.setPixelSize(9);
        f.setBold(true);
        p.setFont(f);
        p.drawText(QRect(0, 0, 20, 10), Qt::AlignCenter, "AI");
        p.drawLine(2, 13, 18, 13);
        p.drawLine(2, 16, 18, 16);
        p.drawLine(2, 19, 13, 19);
        p.end();
        m_aiDescriptionButton->setIcon(QIcon(pm));
        m_aiDescriptionButton->setIconSize(QSize(20, 20));
    }
    connect(m_aiDescriptionButton, &QPushButton::clicked, this, &MainWindow::onGenerateCommitDescription);

    // Skip hooks toggle
    m_skipHooksButton = new QPushButton();
    m_skipHooksButton->setFixedSize(28, 28);
    m_skipHooksButton->setCheckable(true);
    m_skipHooksButton->setFlat(true);
    m_skipHooksButton->setToolTip("Skip pre-commit hooks");
    {
        QIcon icon = QIcon::fromTheme("media-skip-forward");
        if (icon.isNull())
            icon = style()->standardIcon(QStyle::SP_MediaSkipForward);
        m_skipHooksButton->setIcon(icon);
        m_skipHooksButton->setIconSize(QSize(18, 18));
    }
    m_skipHooksButton->setStyleSheet(
        "QPushButton { border: none; }"
        "QPushButton:checked { background: #264f78; border-radius: 4px; }");

    // Co-author button
    m_coAuthorButton = new QPushButton();
    m_coAuthorButton->setFixedSize(28, 28);
    m_coAuthorButton->setFlat(true);
    m_coAuthorButton->setToolTip("Add co-authors from changes");
    connect(m_coAuthorButton, &QPushButton::clicked, this, &MainWindow::onAddCoAuthors);
    {
        QIcon icon = QIcon::fromTheme("system-users");
        if (icon.isNull())
            icon = style()->standardIcon(QStyle::SP_ComputerIcon);
        m_coAuthorButton->setIcon(icon);
        m_coAuthorButton->setIconSize(QSize(18, 18));
    }

    auto *aiRow = new QHBoxLayout();
    aiRow->setContentsMargins(0, 0, 0, 0);
    aiRow->addWidget(m_aiCommitButton);
    aiRow->addWidget(m_aiDescriptionButton);
    aiRow->addWidget(m_skipHooksButton);
    aiRow->addWidget(m_coAuthorButton);
    aiRow->addStretch();
    m_aiRowContainer = new QWidget();
    m_aiRowContainer->setLayout(aiRow);
    commitBodyLayout->addWidget(m_aiRowContainer);

    // AI row starts hidden; visibility checked asynchronously after setupUi
    m_aiRowContainer->hide();

    // AI thinking indicator overlay
    m_aiThinkingOverlay = new QWidget(m_commitContainer);
    m_aiThinkingOverlay->setVisible(false);
    m_aiThinkingOverlay->setStyleSheet(
        "background: rgba(30, 30, 30, 0.95);"
        "border: 1px solid #0e639c;"
        "border-radius: 4px;"
    );
    auto *thinkingLayout = new QVBoxLayout(m_aiThinkingOverlay);
    thinkingLayout->setContentsMargins(8, 8, 8, 8);
    thinkingLayout->setSpacing(4);

    m_aiThinkingLabel = new QLabel("AI is thinking...");
    m_aiThinkingLabel->setStyleSheet("color: #d4d4d4; font-weight: bold;");
    thinkingLayout->addWidget(m_aiThinkingLabel);

    m_aiShowMoreButton = new QPushButton("Show more ▼");
    m_aiShowMoreButton->setFlat(true);
    m_aiShowMoreButton->setStyleSheet(
        "QPushButton { border: none; color: #0e639c; text-align: left; padding: 4px; }"
        "QPushButton:hover { color: #1177bb; }"
    );
    thinkingLayout->addWidget(m_aiShowMoreButton);

    m_aiThinkingText = new QTextEdit();
    m_aiThinkingText->setVisible(false);
    m_aiThinkingText->setReadOnly(true);
    m_aiThinkingText->setMaximumHeight(200);
    m_aiThinkingText->setStyleSheet(
        "background: #1e1e1e;"
        "color: #d4d4d4;"
        "border: 1px solid #3c3c3c;"
        "font-family: monospace;"
        "font-size: 11px;"
    );
    thinkingLayout->addWidget(m_aiThinkingText);

    connect(m_aiShowMoreButton, &QPushButton::clicked, this, [this]() {
        m_aiThinkingVisible = !m_aiThinkingVisible;
        m_aiThinkingText->setVisible(m_aiThinkingVisible);
        m_aiShowMoreButton->setText(m_aiThinkingVisible ? "Show less ▲" : "Show more ▼");
    });

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

    // Single file tree with header bar
    auto *changesContentLayout = new QVBoxLayout();
    changesContentLayout->setContentsMargins(0, 0, 0, 0);
    changesContentLayout->setSpacing(0);
    changesContentLayout->addWidget(m_headerBar);
    changesContentLayout->addWidget(m_gitStatusTree, 1);
    changesContentLayout->addWidget(commitContainer);

    changesLayout->addLayout(changesContentLayout);
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
    auto *cfCloseBtn = new QPushButton();
    cfCloseBtn->setFixedSize(20, 20);
    cfCloseBtn->setFlat(true);
    cfCloseBtn->setIcon(style()->standardIcon(QStyle::SP_DialogCloseButton));
    cfCloseBtn->setIconSize(QSize(12, 12));
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
    // m_addProjectButton uses setMenu() instead of clicked
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

    m_fsWatcher = new QFileSystemWatcher(this);
    m_refreshTimer = new QTimer(this);
    m_refreshTimer->setSingleShot(true);
    m_refreshTimer->setInterval(2000);
    connect(m_fsWatcher, &QFileSystemWatcher::directoryChanged,
            this, &MainWindow::onRepoDirChanged);
    connect(m_fsWatcher, &QFileSystemWatcher::fileChanged,
            this, &MainWindow::onRepoDirChanged);
    connect(m_refreshTimer, &QTimer::timeout,
            this, &MainWindow::onRefreshDebounce);

    m_networkManager = new QNetworkAccessManager(this);

    // Check AI availability asynchronously
    QTimer::singleShot(0, this, [this]() {
        auto s = lazySettings();
        bool aiEnabled = s->value("ai/enabled", false).toBool();
        if (!aiEnabled) {
            m_aiRowContainer->hide();
            return;
        }
        bool isLocal = s->value("ai/provider", "OpenRouter").toString() == "Local (internal llama.cpp)";
        bool hasKey = !s->value("ai/api_key").toString().isEmpty();
        if (isLocal || hasKey)
            m_aiRowContainer->show();
    });
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
    auto readUrl = [&](const QStringList &args) -> QString {
        QProcess proc;
        proc.setWorkingDirectory(path);
        proc.start("git", args);
        if (!proc.waitForFinished(3000) || proc.exitCode() != 0)
            return {};
        return QString::fromUtf8(proc.readAllStandardOutput()).trimmed();
    };

    QString url = readUrl({"config", "--local", "remote.origin.url"});
    if (url.isEmpty())
        url = readUrl({"remote", "get-url", "origin"});
    if (url.isEmpty())
        return {};

    if (url.endsWith(".git"))
        url.chop(4);

    // Normalize SSH: git@github.com:owner/repo → https://github.com/owner/repo
    if (url.startsWith("git@")) {
        url.remove(0, 4);
        url.replace(':', '/');
        url.prepend("https://");
    }

    // Extract owner: the path segment right after the host
    // Works for https://host/owner/repo, ssh://git@host/owner/repo, etc.
    QRegularExpression re("^[a-z]+://[^/]+/([^/]+)");
    auto m = re.match(url);
    if (m.hasMatch())
        return m.captured(1);

    return {};
}

// --- Recent projects persistence ---

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

void MainWindow::onClearAllProjects()
{
    if (m_recentProjects.isEmpty())
        return;

    auto reply = QMessageBox::question(this, "Remove All Projects",
        "Remove all projects from the list?\n\n"
        "This will not delete any directories.",
        QMessageBox::Yes | QMessageBox::No, QMessageBox::No);

    if (reply != QMessageBox::Yes)
        return;

    closeRepository();
    m_recentProjects.clear();
    saveRecentProjects();
    populateRecentList();
}

void MainWindow::onCloneRepository()
{
    bool ok = false;
    QString url = QInputDialog::getText(this, "Clone Repository",
        "Git remote URL:", QLineEdit::Normal, {}, &ok);
    if (!ok || url.trimmed().isEmpty())
        return;

    QString dest = QFileDialog::getExistingDirectory(this, "Destination Directory");
    if (dest.isEmpty())
        return;

    // Derive folder name from URL (e.g. "owner/repo" → "repo", "repo.git" → "repo")
    QString repoName = url.section('/', -1);
    if (repoName.endsWith(".git"))
        repoName.chop(4);
    dest = dest + "/" + repoName;

    QProcess proc;
    proc.setWorkingDirectory(QFileInfo(dest).absolutePath());
    proc.start("git", {"clone", url.trimmed(), dest});
    proc.setProcessChannelMode(QProcess::MergedChannels);

    QMessageBox info(this);
    info.setWindowTitle("Cloning");
    info.setText("Cloning " + repoName + "…");
    info.setStandardButtons(QMessageBox::NoButton);
    info.show();

    if (!proc.waitForFinished(120000) || proc.exitCode() != 0) {
        info.done(0);
        QMessageBox::warning(this, "Clone Failed",
            "Failed to clone repository:\n" + QString::fromUtf8(proc.readAll()));
        return;
    }
    info.done(0);

    m_recentProjects.removeAll(dest);
    m_recentProjects.prepend(dest);
    saveRecentProjects();
    populateRecentList();
    m_recentDrawer->setVisible(false);
    openRepository(dest);
}

void MainWindow::onCreateRepository()
{
    QString dir = QFileDialog::getExistingDirectory(this, "Directory for New Repository");
    if (dir.isEmpty())
        return;

    QMessageBox info(this);
    info.setWindowTitle("Creating");
    info.setText("Initializing git repository…");
    info.setStandardButtons(QMessageBox::NoButton);
    info.show();

    QProcess proc;
    proc.setWorkingDirectory(dir);
    proc.start("git", {"init"});
    if (!proc.waitForFinished(10000) || proc.exitCode() != 0) {
        info.done(0);
        QMessageBox::warning(this, "Init Failed",
            "Failed to initialize git repository:\n"
            + QString::fromUtf8(proc.readAll()));
        return;
    }
    info.done(0);

    m_recentProjects.removeAll(dir);
    m_recentProjects.prepend(dir);
    saveRecentProjects();
    populateRecentList();
    m_recentDrawer->setVisible(false);
    openRepository(dir);
}

void MainWindow::onOpenExistingProject()
{
    const QString dir = QFileDialog::getExistingDirectory(
        this, "Open Git Repository");
    if (!dir.isEmpty())
        openRepository(dir);
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

    if (m_recentDrawer->isVisible()) {
        m_recentDrawer->hide();
        return;
    }

    auto *cw = centralWidget();
    int toolbarH = m_projectButton->mapTo(cw, QPoint(0, 0)).y()
                 + m_projectButton->height() + 2;
    m_recentDrawer->setGeometry(0, toolbarH, 260,
        cw->height() - toolbarH);
    m_recentDrawer->raise();
    m_recentDrawer->show();
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
    m_aiCommitButton->setEnabled(true);
    m_aiDescriptionButton->setEnabled(true);
    m_skipHooksButton->setEnabled(true);
    m_coAuthorButton->setEnabled(true);
    loadBranches();
    startGitStatusQuery();
    startGitLogQuery();

    // Watch for file changes to auto-refresh
    QStringList currentFiles = m_fsWatcher->files();
    QStringList currentDirs = m_fsWatcher->directories();
    if (!currentFiles.isEmpty())
        m_fsWatcher->removePaths(currentFiles);
    if (!currentDirs.isEmpty())
        m_fsWatcher->removePaths(currentDirs);
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
    m_aiCommitButton->setEnabled(false);
    m_aiDescriptionButton->setEnabled(false);
    m_skipHooksButton->setEnabled(false);
    m_coAuthorButton->setEnabled(false);
    m_currentBranch.clear();
    m_selectedCommitHash.clear();

    // Safely remove paths from watcher
    QStringList files = m_fsWatcher->files();
    QStringList dirs = m_fsWatcher->directories();
    if (!files.isEmpty())
        m_fsWatcher->removePaths(files);
    if (!dirs.isEmpty())
        m_fsWatcher->removePaths(dirs);
}

void MainWindow::onOpenEditor()
{
    if (m_repoPath.isEmpty())
        return;

    // Open in the platform's default editor/IDE
#if defined(Q_OS_WIN)
    static const char *editors[] = {"code", "notepad++", "notepad", nullptr};
    for (const char **ed = editors; *ed; ++ed) {
        if (!QStandardPaths::findExecutable(*ed).isEmpty()) {
            QProcess::startDetached(*ed, {m_repoPath});
            return;
        }
    }
    QProcess::startDetached("explorer", {m_repoPath});
#elif defined(Q_OS_MACOS)
    static const char *editors[] = {"code", "subl", "TextEdit", nullptr};
    for (const char **ed = editors; *ed; ++ed) {
        if (!QStandardPaths::findExecutable(*ed).isEmpty()) {
            QProcess::startDetached("open", {"-a", *ed, m_repoPath});
            return;
        }
    }
    QProcess::startDetached("open", {m_repoPath});
#else
    static const char *editors[] = {"code", "gedit", "kate", "subl", "xed", "mousepad", nullptr};
    for (const char **ed = editors; *ed; ++ed) {
        if (!QStandardPaths::findExecutable(*ed).isEmpty()) {
            QProcess::startDetached(*ed, {m_repoPath});
            return;
        }
    }
    QDesktopServices::openUrl(QUrl::fromLocalFile(m_repoPath));
#endif
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

#if defined(Q_OS_WIN)
    QProcess::startDetached("cmd.exe", {"/C", "start", "cmd", "/K", "cd", "/D", m_repoPath});
#elif defined(Q_OS_MACOS)
    QProcess::startDetached("open", {"-a", "Terminal", m_repoPath});
#else
    // Linux: try known terminal emulators
    static const struct { const char *bin; const char *flag; } terms[] = {
        {"x-terminal-emulator", nullptr},
        {"konsole", "--workdir"},
        {"gnome-terminal", "--working-directory"},
        {"xfce4-terminal", "--working-directory"},
        {"lxterminal", "--working-directory"},
        {"alacritty", "--working-directory"},
        {"kitty", "--working-directory"},
        {"xterm", nullptr},
        {nullptr, nullptr},
    };
    for (const auto *t = terms; t->bin; ++t) {
        QString exec = QStandardPaths::findExecutable(QLatin1String(t->bin));
        if (exec.isEmpty())
            continue;
        if (t->flag)
            QProcess::startDetached(QLatin1String(t->bin),
                {QLatin1String(t->flag), m_repoPath});
        else
            QProcess::startDetached(QLatin1String(t->bin), {m_repoPath});
        return;
    }
#endif
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

// --- Custom YAML themes ---

struct Theme {
    QString name;
    QMap<QString, QString> colors;
};


static QList<Theme> loadCustomThemes()
{
    QList<Theme> list;
    QDir dir(themesDirPath());
    if (!dir.exists())
        return list;
    const QStringList files = dir.entryList({"*.theme.yaml"}, QDir::Files, QDir::Name);
    for (const QString &fn : files) {
        try {
            YAML::Node root = YAML::LoadFile(dir.filePath(fn).toStdString());
            if (!root["name"] || !root["colors"])
                continue;
            Theme t;
            t.name = QString::fromStdString(root["name"].as<std::string>());
            auto colors = root["colors"];
            for (auto it = colors.begin(); it != colors.end(); ++it) {
                QString key = QString::fromStdString(it->first.as<std::string>());
                QString val = QString::fromStdString(it->second.as<std::string>());
                t.colors[key] = val;
            }
            list.append(t);
        } catch (...) {
            continue;
        }
    }
    return list;
}

static QString generateStylesheet(const Theme &t)
{
    auto c = [&](const QString &k, const QString &fallback) -> QString {
        return t.colors.value(k, fallback);
    };
    return QString(
        "QWidget { background-color: %1; color: %2; }"
        "QTreeWidget, QListWidget { background-color: %3; }"
        "QPushButton { background-color: %4; color: %5; }"
        "QLineEdit, QTextEdit { background-color: %6; color: %7; }"
        "QToolTip { background-color: %8; color: %9; }")
        .arg(c("background", "#1e1e1e"))
        .arg(c("foreground", "#d4d4d4"))
        .arg(c("widget_background", "#252526"))
        .arg(c("button_background", "#0e639c"))
        .arg(c("button_foreground", "white"))
        .arg(c("input_background", "#3c3c3c"))
        .arg(c("input_foreground", "#d4d4d4"))
        .arg(c("tooltip_background", "#3c3c3c"))
        .arg(c("tooltip_foreground", "#d4d4d4"));
}

static Theme findTheme(const QString &name, const QList<Theme> &customs)
{
    for (const auto &t : customs)
        if (t.name == name) return t;
    return {};
}

void MainWindow::applySavedTheme()
{
    auto settings = lazySettings();
    auto *app = qobject_cast<QApplication *>(qApp);
    if (!app)
        return;

    QString theme = settings->value("appearance/theme", "system").toString();

    if (theme == "system") {
        app->setStyleSheet({});
    } else if (theme == "dark") {
        app->setStyleSheet(
            "QWidget { background-color: #1e1e1e; color: #d4d4d4; }"
            "QTreeWidget, QListWidget { background-color: #252526; }"
            "QPushButton { background-color: #0e639c; color: white; }"
            "QLineEdit, QTextEdit { background-color: #3c3c3c; color: #d4d4d4; }"
            "QToolTip { background-color: #3c3c3c; color: #d4d4d4; }");
    } else {
        // Custom theme by name
        Theme t = findTheme(theme, loadCustomThemes());
        if (t.name.isEmpty())
            app->setStyleSheet({});
        else
            app->setStyleSheet(generateStylesheet(t));
    }
}

void MainWindow::onGenerateCommitMessage()
{
    auto settings = lazySettings();
    const QString rawPrompt = settings->value("ai/system_prompt",
        "Generate a Conventional Commits summary and a casual description of all changes.")
        .toString();
    runAiGeneration(AiRequestKind::CommitMessage, rawPrompt);
}

void MainWindow::onGenerateCommitDescription()
{
    auto settings = lazySettings();
    const QString rawPrompt = settings->value("ai/description_system_prompt",
        kDefaultDescriptionSystemPrompt).toString();
    runAiGeneration(AiRequestKind::Description, rawPrompt);
}

void MainWindow::runAiGeneration(AiRequestKind kind, const QString &rawPrompt)
{
    auto settings = lazySettings();
    const QString provider = settings->value("ai/provider", "OpenRouter").toString();
    const QString model = settings->value("ai/model", "gpt-4o-mini").toString();

    bool isLocal = provider == "Local (internal llama.cpp)";

    QString apiKey;
    if (!isLocal) {
        apiKey = settings->value("ai/api_key").toString();
        if (apiKey.isEmpty()) {
            QMessageBox::information(this, "API Key Required",
                "No API key configured.\n\n"
                "Go to Settings \u2192 AI to add one.");
            return;
        }
    }

    QStringList files = checkedFiles();
    if (files.isEmpty()) {
        QMessageBox::information(this, "No Files Selected",
            "Check at least one file to generate a commit message.");
        return;
    }

    QString diffText = buildDiffText(m_repoPath, files);
    AiPrompt prompt = buildAiPrompt(rawPrompt, diffText);
    const QString &systemPrompt = prompt.systemPrompt;
    const QString &userContent = prompt.userContent;

    m_aiRequestKind = kind;

    // Handle local llama.cpp provider
    if (provider == "Local (internal llama.cpp)") {
        QString modelPath = settings->value("ai/local_model_path").toString();
        if (modelPath.isEmpty()) {
            QMessageBox::information(this, "Model Required",
                "No local model selected.\n\n"
                "Go to Settings \u2192 AI to download and select a GGUF model.");
            return;
        }

        if (!QFileInfo::exists(modelPath)) {
            QMessageBox::warning(this, "Model Not Found",
                "The selected model file was not found:\n" + modelPath + "\n\n"
                "The file may have been moved or deleted. Go to Settings \u2192 AI to select a different model.");
            return;
        }

        m_aiCommitButton->setEnabled(false);
        m_aiDescriptionButton->setEnabled(false);
        m_summaryInput->setEnabled(false);
        m_descriptionInput->setEnabled(false);

        if (m_aiThinkingOverlay && m_commitContainer) {
            m_aiThinkingOverlay->setGeometry(m_commitContainer->rect());
            m_aiThinkingOverlay->setVisible(true);
            m_aiThinkingText->clear();
            m_aiThinkingText->setVisible(false);
            m_aiShowMoreButton->setText("Show more \u25bc");
            m_aiThinkingVisible = false;
        }

        int nGpuLayers = settings->value("ai/gpu_acceleration", true).toBool() ? 99 : 0;

        const QString vcs = detectVcsKind(m_repoPath);
        const QString mode = kind == AiRequestKind::Description
                                 ? QStringLiteral("description")
                                 : QStringLiteral("message");

        QJsonArray recent;
        for (const QString &msg : recentCommitMessages(m_repoPath, vcs))
            recent.append(msg);

        QJsonObject context;
        context["vcs"] = vcs;
        context["repo_path"] = m_repoPath;
        context["diff"] = diffText;
        context["files"] = fileStatusesJson(m_repoPath, vcs, files);
        context["staged"] = QJsonArray::fromStringList(stagedFiles(m_repoPath, vcs, files));
        context["branch"] = currentBranchOrChange(m_repoPath, vcs);
        context["recent_messages"] = recent;
        context["mode"] = mode;
        context["system_prompt"] = rawPrompt;

        m_modelManager->generateCommitMessage(modelPath, context, nGpuLayers);
        return;
    }

    m_aiCommitButton->setEnabled(false);
    m_aiDescriptionButton->setEnabled(false);
    m_summaryInput->setEnabled(false);
    m_descriptionInput->setEnabled(false);

    // Show thinking indicator
    if (m_aiThinkingOverlay && m_commitContainer) {
        m_aiThinkingOverlay->setGeometry(m_commitContainer->rect());
        m_aiThinkingOverlay->setVisible(true);
        m_aiThinkingText->clear();
        m_aiThinkingText->setVisible(false);
        m_aiShowMoreButton->setText("Show more \u25bc");
        m_aiThinkingVisible = false;
    }

    QJsonObject msgSystem, msgUser;
    msgSystem["role"] = "system";
    msgSystem["content"] = systemPrompt;
    msgUser["role"] = "user";
    msgUser["content"] = userContent;

    QJsonArray messages;
    messages.append(msgSystem);
    messages.append(msgUser);

    QJsonObject body;
    body["model"] = model;
    body["messages"] = messages;

    // Determine endpoint and auth based on provider
    QUrl url;
    QByteArray authHeader;

    if (provider == "OpenRouter") {
        url = "https://openrouter.ai/api/v1/chat/completions";
        authHeader = "Bearer " + apiKey.toUtf8();
    } else if (provider == "OpenAI") {
        url = "https://api.openai.com/v1/chat/completions";
        authHeader = "Bearer " + apiKey.toUtf8();
    } else if (provider == "Anthropic") {
        url = "https://api.anthropic.com/v1/messages";
        // Anthropic uses a different request format
        body.remove("messages");
        QJsonObject anonMsg;
        anonMsg["role"] = "user";
        anonMsg["content"] = "System: " + systemPrompt + "\n\n" + userContent;
        QJsonArray anonMessages;
        anonMessages.append(anonMsg);
        body["messages"] = anonMessages;
        body["max_tokens"] = 1024;
    } else if (provider == "Google AI Studio") {
        url = "https://generativelanguage.googleapis.com/v1beta/models/"
              + model + ":generateContent?key=" + apiKey;
        body.remove("messages");
        QJsonArray contents;
        QJsonObject part;
        part["text"] = "System: " + systemPrompt + "\n\n" + userContent;
        QJsonArray parts;
        parts.append(part);
        QJsonObject content;
        content["role"] = "user";
        content["parts"] = parts;
        contents.append(content);
        body["contents"] = contents;
        authHeader.clear();
    }

    QNetworkRequest req(url);
    req.setHeader(QNetworkRequest::ContentTypeHeader, "application/json");
    if (provider == "Anthropic") {
        req.setRawHeader("x-api-key", apiKey.toUtf8());
        req.setRawHeader("anthropic-version", "2023-06-01");
    } else if (!authHeader.isEmpty()) {
        req.setRawHeader("Authorization", authHeader);
    }

    QByteArray payload = QJsonDocument(body).toJson(QJsonDocument::Compact);
    QNetworkReply *reply = m_networkManager->post(req, payload);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        reply->deleteLater();
        onAiResponse(reply);
    });
}

static QString extractAiText(const QJsonObject &obj, const QString &provider)
{
    if (provider == "Anthropic") {
        QJsonArray c = obj["content"].toArray();
        if (!c.isEmpty())
            return c[0].toObject()["text"].toString().trimmed();
        return {};
    }
    if (provider == "Google AI Studio") {
        QJsonArray cands = obj["candidates"].toArray();
        if (cands.isEmpty()) return {};
        QJsonArray parts = cands[0].toObject()["content"].toObject()["parts"].toArray();
        if (parts.isEmpty()) return {};
        return parts[0].toObject()["text"].toString().trimmed();
    }
    // OpenAI-compatible (OpenRouter, OpenAI)
    QJsonArray choices = obj["choices"].toArray();
    if (choices.isEmpty()) return {};
    return choices[0].toObject()["message"].toObject()["content"].toString().trimmed();
}

void MainWindow::onAiResponse(QNetworkReply *reply)
{
    // Hide thinking indicator
    if (m_aiThinkingOverlay)
        m_aiThinkingOverlay->setVisible(false);

    m_aiCommitButton->setEnabled(true);
    m_aiDescriptionButton->setEnabled(true);
    m_summaryInput->setEnabled(true);
    m_descriptionInput->setEnabled(true);

    if (reply->error() != QNetworkReply::NoError) {
        QMessageBox::warning(this, "AI Request Failed",
            "Failed to generate commit message:\n" + reply->errorString());
        return;
    }

    auto s = lazySettings();
    QString provider = s->value("ai/provider", "OpenRouter").toString();
    QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
    QJsonObject obj = doc.object();
    QString content = extractAiText(obj, provider);
    if (content.isEmpty()) {
        QMessageBox::warning(this, "AI Error", "No response from AI.");
        return;
    }

    if (m_aiRequestKind == AiRequestKind::Description) {
        m_descriptionInput->setPlainText(content.trimmed());
        return;
    }

    QStringList lines = content.split('\n', Qt::SkipEmptyParts);
    if (lines.isEmpty())
        return;

    QString summary = lines.first();
    summary.remove(QRegularExpression("^#+\\s*"));
    summary = summary.trimmed();
    m_summaryInput->setText(summary);

    int descStart = -1;
    for (int i = 0; i < lines.size(); i++) {
        if (lines[i].contains("Casual Description", Qt::CaseInsensitive)) {
            descStart = i + 1;
            break;
        }
    }

    if (descStart > 0 && descStart < lines.size()) {
        QStringList descLines;
        for (int i = descStart; i < lines.size(); i++) {
            if (!lines[i].trimmed().isEmpty())
                descLines << lines[i];
        }
        m_descriptionInput->setPlainText(descLines.join('\n').trimmed());
    }
}

void MainWindow::onAddCoAuthors()
{
    if (m_repoPath.isEmpty())
        return;

    QStringList files = checkedFiles();
    if (files.isEmpty()) {
        QMessageBox::information(this, "No Files Selected",
            "Check at least one file to find co-authors from.");
        return;
    }

    QSet<QString> authors;
    for (const QString &file : files) {
        QProcess p;
        p.setWorkingDirectory(m_repoPath);
        p.start("git", {"log", "--follow", "--format=%an <%ae>", "--", file});
        if (p.waitForFinished(5000) && p.exitCode() == 0) {
            const QStringList lines = QString::fromUtf8(p.readAllStandardOutput())
                .split('\n', Qt::SkipEmptyParts);
            for (const QString &l : lines)
                authors.insert(l.trimmed());
        }
    }

    if (authors.isEmpty()) {
        QMessageBox::information(this, "No Authors Found",
            "Could not find any co-authors for the selected files.");
        return;
    }

    QStringList sorted = authors.values();
    sorted.sort(Qt::CaseInsensitive);

    // Remove current user from the list
    QProcess whoami;
    whoami.start("git", {"config", "user.name"});
    QString currentName;
    if (whoami.waitForFinished(2000) && whoami.exitCode() == 0)
        currentName = QString::fromUtf8(whoami.readAllStandardOutput()).trimmed();
    sorted.erase(std::remove_if(sorted.begin(), sorted.end(),
        [&](const QString &a) { return a.startsWith(currentName); }),
        sorted.end());

    if (sorted.isEmpty()) {
        QMessageBox::information(this, "No Co-Authors",
            "No other authors found for the selected files.");
        return;
    }

    QDialog dlg(this);
    dlg.setWindowTitle("Select Co-Authors");
    dlg.setMinimumWidth(350);
    auto *dlgLayout = new QVBoxLayout(&dlg);
    auto *list = new QListWidget();
    list->setAlternatingRowColors(true);
    for (const QString &a : sorted) {
        auto *item = new QListWidgetItem(a);
        item->setCheckState(Qt::Unchecked);
        list->addItem(item);
    }
    dlgLayout->addWidget(new QLabel("Select co-authors to add to the commit:"));
    dlgLayout->addWidget(list, 1);

    auto *btnBox = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);
    connect(btnBox, &QDialogButtonBox::accepted, &dlg, &QDialog::accept);
    connect(btnBox, &QDialogButtonBox::rejected, &dlg, &QDialog::reject);
    dlgLayout->addWidget(btnBox);

    if (dlg.exec() != QDialog::Accepted)
        return;

    QStringList trailers;
    for (int i = 0; i < list->count(); i++) {
        if (list->item(i)->checkState() == Qt::Checked)
            trailers << "Co-authored-by: " + list->item(i)->text().trimmed();
    }

    if (trailers.isEmpty())
        return;

    // Preserve existing description content
    QString desc = m_descriptionInput->toPlainText().trimmed();
    if (!desc.isEmpty())
        desc += "\n\n";
    desc += trailers.join('\n');
    m_descriptionInput->setPlainText(desc);
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
    categories->addItem("AI");

    auto *stack = new QStackedWidget();

    auto settings = lazySettings();

    // General page
    auto *generalPage = new QWidget();
    auto *generalLayout = new QVBoxLayout(generalPage);
    generalLayout->setContentsMargins(12, 12, 12, 12);
    generalLayout->setSpacing(8);

    auto *infoLabel = new QLabel("Data locations (changes apply after restart):");
    infoLabel->setStyleSheet("font-weight: bold;");
    generalLayout->addWidget(infoLabel);

    auto *projectsPathInput = new QLineEdit();
    projectsPathInput->setText(projectsFilePath());
    auto *settingsPathInput = new QLineEdit();
    settingsPathInput->setText(settingsFilePath());
    auto *themesPathInput = new QLineEdit();
    themesPathInput->setText(themesDirPath());

    auto addPathRow = [&](const QString &label, QLineEdit *input) {
        auto *row = new QHBoxLayout();
        auto *hdr = new QLabel(label);
        hdr->setStyleSheet("color: gray;");
        hdr->setFixedWidth(80);
        row->addWidget(hdr);
        row->addWidget(input, 1);
        generalLayout->addLayout(row);
    };
    addPathRow("Projects:", projectsPathInput);
    addPathRow("Settings:", settingsPathInput);
    addPathRow("Themes:", themesPathInput);

    generalLayout->addStretch();

    // Appearance page
    auto *appearancePage = new QWidget();
    auto *appearanceLayout = new QFormLayout(appearancePage);
    appearanceLayout->setContentsMargins(12, 12, 12, 12);
    appearanceLayout->setSpacing(8);

    auto *themeCombo = new QComboBox();
    themeCombo->addItem("System Default");
    themeCombo->addItem("Dark");

    QList<Theme> customThemes = loadCustomThemes();
    if (!customThemes.isEmpty()) {
        themeCombo->insertSeparator(themeCombo->count());
        for (const auto &t : customThemes)
            themeCombo->addItem(t.name);
    }

    QString storedTheme = settings->value("appearance/theme", "system").toString();
    if (storedTheme == "dark")
        themeCombo->setCurrentIndex(1);
    else if (storedTheme == "system")
        themeCombo->setCurrentIndex(0);
    else {
        int ci = themeCombo->findText(storedTheme);
        if (ci >= 0) themeCombo->setCurrentIndex(ci);
    }
    appearanceLayout->addRow("Theme:", themeCombo);

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

    // AI page
    auto *aiPage = new QWidget();
    auto *aiLayout = new QVBoxLayout(aiPage);
    aiLayout->setContentsMargins(12, 12, 12, 12);
    aiLayout->setSpacing(8);

    // 1. Enable AI Features toggle
    auto *aiEnableCheck = new QCheckBox("Enable AI Features");
    aiEnableCheck->setChecked(settings->value("ai/enabled", false).toBool());
    aiLayout->addWidget(aiEnableCheck);

    // 2. Provider dropdown
    auto *aiProviderLabel = new QLabel("Provider:");
    auto *aiProviderCombo = new QComboBox();
    aiProviderCombo->addItems({"OpenRouter", "OpenAI", "Anthropic",
                               "Google AI Studio", "Local (internal llama.cpp)"});
    QString currentProvider = settings->value("ai/provider", "OpenRouter").toString();
    int providerIndex = aiProviderCombo->findText(currentProvider);
    if (providerIndex >= 0)
        aiProviderCombo->setCurrentIndex(providerIndex);
    aiLayout->addWidget(aiProviderLabel);
    aiLayout->addWidget(aiProviderCombo);

    // 3. API Key field
    auto *aiKeyLabel = new QLabel("API Key:");
    auto *apiKeyInput = new QLineEdit();
    apiKeyInput->setPlaceholderText("sk-...");
    apiKeyInput->setEchoMode(QLineEdit::Password);
    QString storedKey = settings->value("ai/api_key").toString();
    if (!storedKey.isEmpty())
        apiKeyInput->setText(storedKey);
    aiLayout->addWidget(aiKeyLabel);
    aiLayout->addWidget(apiKeyInput);

    // 4. Model field (editable combo)
    auto *aiModelLabel = new QLabel("Model:");
    auto *aiModelCombo = new QComboBox();
    aiModelCombo->setEditable(true);
    aiModelCombo->setInsertPolicy(QComboBox::NoInsert);
    aiModelCombo->setMinimumWidth(200);
    QString savedModel = settings->value("ai/model").toString();
    if (!savedModel.isEmpty())
        aiModelCombo->setCurrentText(savedModel);
    aiLayout->addWidget(aiModelLabel);
    aiLayout->addWidget(aiModelCombo);

    // Model fetching logic
    auto doFetchModels = [this, aiProviderCombo, aiModelCombo, apiKeyInput]() {
        QString provider = aiProviderCombo->currentText();
        QString key = apiKeyInput->text().trimmed();

        // Nothing to fetch for OpenRouter or Local
        if (provider == "OpenRouter" || provider == "Local (internal llama.cpp)")
            return;

        if (key.isEmpty())
            return;

        aiModelCombo->clear();
        aiModelCombo->setCurrentText(QString());
        aiModelCombo->setPlaceholderText("Loading models...");

        QUrl url;
        QNetworkRequest req;
        if (provider == "OpenAI") {
            url = QUrl("https://api.openai.com/v1/models");
            req.setRawHeader("Authorization", "Bearer " + key.toUtf8());
        } else if (provider == "Anthropic") {
            url = QUrl("https://api.anthropic.com/v1/models");
            req.setRawHeader("x-api-key", key.toUtf8());
            req.setRawHeader("anthropic-version", "2023-06-01");
        } else if (provider == "Google AI Studio") {
            url = QUrl("https://generativelanguage.googleapis.com/v1beta/models?key=" + key);
        }

        req.setUrl(url);
        auto *reply = m_networkManager->get(req);
        connect(reply, &QNetworkReply::finished, this, [reply, aiModelCombo, provider]() {
            reply->deleteLater();
            if (reply->error() != QNetworkReply::NoError) {
                aiModelCombo->setPlaceholderText("Failed to load models");
                return;
            }
            QJsonDocument doc = QJsonDocument::fromJson(reply->readAll());
            QStringList modelNames;
            if (provider == "OpenAI") {
                QJsonArray data = doc.object()["data"].toArray();
                for (const auto &entry : data)
                    modelNames << entry.toObject()["id"].toString();
            } else if (provider == "Anthropic") {
                QJsonArray data = doc.object()["data"].toArray();
                for (const auto &entry : data)
                    modelNames << entry.toObject()["id"].toString();
            } else if (provider == "Google AI Studio") {
                QJsonArray models = doc.object()["models"].toArray();
                for (const auto &entry : models) {
                    QString name = entry.toObject()["name"].toString();
                    name.remove(QRegularExpression("^models/"));
                    modelNames << name;
                }
            }
            modelNames.sort();
            aiModelCombo->clear();
            aiModelCombo->addItems(modelNames);
            aiModelCombo->setPlaceholderText(QString());
            if (modelNames.isEmpty())
                aiModelCombo->setPlaceholderText("No models found");
        });
    };

    // Handle provider changes
    connect(aiProviderCombo, &QComboBox::currentTextChanged, this,
            [aiKeyLabel, aiKeyInput = apiKeyInput, aiModelCombo, aiProviderCombo, doFetchModels]() {
        QString provider = aiProviderCombo->currentText();
        bool isLocal = provider == "Local (internal llama.cpp)";

        aiKeyInput->setEnabled(!isLocal);
        aiModelCombo->setEnabled(!isLocal);

        if (isLocal) {
            aiModelCombo->clear();
            aiModelCombo->setCurrentText(QString());
            aiModelCombo->setPlaceholderText("Configured separately below");
        } else if (provider == "OpenRouter") {
            aiModelCombo->clear();
            aiModelCombo->setCurrentText("deepseek/deepseek-v4-flash");
            aiModelCombo->setEnabled(true);
        } else {
            aiModelCombo->clear();
            aiModelCombo->setCurrentText(QString());
            aiModelCombo->setEnabled(true);
            doFetchModels();
        }
    });

    // Re-fetch models when API key focus is lost
    connect(apiKeyInput, &QLineEdit::editingFinished, this, [doFetchModels, aiProviderCombo]() {
        QString provider = aiProviderCombo->currentText();
        if (provider != "OpenRouter" && provider != "Local (internal llama.cpp)")
            doFetchModels();
    });

    // 5. Enable/disable all AI fields when toggle changes
    connect(aiEnableCheck, &QCheckBox::toggled, this, [aiProviderLabel,
            aiKeyLabel, apiKeyInput, aiModelLabel, aiModelCombo, aiEnableCheck,
            aiProviderCombo2 = aiProviderCombo]() {
        bool enabled = aiEnableCheck->isChecked();
        bool isLocal = aiProviderCombo2->currentText() == "Local (internal llama.cpp)";
        aiProviderLabel->setEnabled(enabled);
        aiProviderCombo2->setEnabled(enabled);
        aiKeyLabel->setEnabled(enabled);
        apiKeyInput->setEnabled(enabled && !isLocal);
        aiModelLabel->setEnabled(enabled);
        aiModelCombo->setEnabled(enabled && !isLocal);
    });

    // Apply initial enabled state
    if (!aiEnableCheck->isChecked()) {
        aiProviderLabel->setEnabled(false);
        aiProviderCombo->setEnabled(false);
        aiKeyLabel->setEnabled(false);
        apiKeyInput->setEnabled(false);
        aiModelLabel->setEnabled(false);
        aiModelCombo->setEnabled(false);
    } else {
        // Apply initial provider-based state
        bool isLocal = aiProviderCombo->currentText() == "Local (internal llama.cpp)";
        apiKeyInput->setEnabled(!isLocal);
        if (isLocal) {
            aiModelCombo->clear();
            aiModelCombo->setPlaceholderText("Configured separately below");
            aiModelCombo->setEnabled(false);
        } else if (aiProviderCombo->currentText() == "OpenRouter") {
            aiModelCombo->clear();
            aiModelCombo->setCurrentText(
                savedModel.isEmpty() ? "deepseek/deepseek-v4-flash" : savedModel);
        }
    }

    // 6. System prompt
    static const QString kDefaultSystemPrompt = QStringLiteral(
        "You are a CLI tool that outputs exactly ONE single conventional commit message summarizing the entire diff.\n"
        "Do not write a separate commit for each file. Find the highest-level feature or fix and summarize it in one line. Do not explain.\n"
        "\n"
        "Diff:\n"
        "--- a/package.json\n"
        "+++ b/package.json\n"
        "@@ -10 +10,2 @@\n"
        "+ \"cors\": \"^2.8.5\"\n"
        "--- a/src/server.js\n"
        "+++ b/src/server.js\n"
        "@@ -2 +2,3 @@\n"
        "+ const cors = require('cors');\n"
        "+ app.use(cors());\n"
        "\n"
        "Commit: feat(api): add cors support to server\n"
        "\n"
        "Diff:\n"
        "<diff>\n"
        "\n"
        "Commit:");

    auto *aiPromptLabel = new QLabel("Commit message system prompt:");
    auto *aiPromptHint = new QLabel("\"<diff>\" will be replaced with the actual git diff");
    aiPromptHint->setStyleSheet("color: gray; font-size: 11px;");
    auto *aiPromptInput = new QPlainTextEdit();
    aiPromptInput->setPlainText(settings->value("ai/system_prompt", kDefaultSystemPrompt).toString());
    aiPromptInput->setFixedHeight(120);
    aiLayout->addWidget(aiPromptLabel);
    aiLayout->addWidget(aiPromptInput, 1);
    aiLayout->addWidget(aiPromptHint);

    auto *aiDescriptionPromptLabel = new QLabel("Commit description system prompt:");
    auto *aiDescriptionPromptHint = new QLabel("\"<diff>\" will be replaced with the actual git diff");
    aiDescriptionPromptHint->setStyleSheet("color: gray; font-size: 11px;");
    auto *aiDescriptionPromptInput = new QPlainTextEdit();
    aiDescriptionPromptInput->setPlainText(
        settings->value("ai/description_system_prompt", kDefaultDescriptionSystemPrompt).toString());
    aiDescriptionPromptInput->setFixedHeight(120);
    aiLayout->addWidget(aiDescriptionPromptLabel);
    aiLayout->addWidget(aiDescriptionPromptInput, 1);
    aiLayout->addWidget(aiDescriptionPromptHint);
    aiLayout->addStretch();

    // 7. Local model configuration
    auto *localSectionLabel = new QLabel("Local model configuration");
    localSectionLabel->setStyleSheet("font-weight: bold; font-size: 13px;");
    aiLayout->addWidget(localSectionLabel);

    auto *localSectionDesc = new QLabel("Download and manage GGUF models for use with the local llama.cpp provider.");
    localSectionDesc->setWordWrap(true);
    localSectionDesc->setStyleSheet("color: gray; font-size: 11px;");
    aiLayout->addWidget(localSectionDesc);

    auto *modelScroll = new QScrollArea();
    modelScroll->setWidgetResizable(true);
    modelScroll->setMaximumHeight(180);
    modelScroll->setFrameStyle(QFrame::StyledPanel);
    modelScroll->setStyleSheet("QScrollArea { border: 1px solid palette(mid); border-radius: 4px; }");
    auto *modelListWidget = new QWidget();
    auto *modelListLayout = new QVBoxLayout(modelListWidget);
    modelListLayout->setContentsMargins(4, 4, 4, 4);
    modelListLayout->setSpacing(4);

    QMap<QString, int> downloadIds;
    QMap<int, QProgressBar *> downloadBars;

    // Single connection for all download progress updates
    connect(
        m_modelManager, &ModelManagerBridge::downloadProgress,
        this, [&downloadBars](int id, qint64 recv, qint64 total) {
            if (auto *bar = downloadBars.value(id)) {
                if (total > 0) {
                    int pct = static_cast<int>(100 * recv / total);
                    bar->setValue(pct);
                    bar->setFormat(QString("%1%").arg(pct));
                } else {
                    bar->setFormat(QString("%1 MB").arg(recv / 1048576));
                }
            }
        });

    std::function<void()> refreshModelList;
    refreshModelList = [&downloadIds, &downloadBars, dlg = &dialog, modelListWidget,
                        modelListLayout, settings = settings.get(), this, &refreshModelList]() {
        QLayoutItem *child;
        while ((child = modelListLayout->takeAt(0)) != nullptr) {
            if (child->widget())
                delete child->widget();
            delete child;
        }

        QString activePath = settings->value("ai/local_model_path").toString();
        QJsonArray models = m_modelManager ? m_modelManager->listLocalModels() : QJsonArray();

        for (const auto &val : models) {
            QJsonObject obj = val.toObject();
            QString name = obj["name"].toString();
            QString url = obj["url"].toString();
            QString path = obj["path"].toString();
            QString sizeLabel = obj["size_label"].toString();
            bool downloaded = obj["downloaded"].toBool();
            bool isActive = !activePath.isEmpty() && path == activePath;

            auto *row = new QWidget();
            auto *rowLayout = new QHBoxLayout(row);
            rowLayout->setContentsMargins(4, 2, 4, 2);
            rowLayout->setSpacing(6);

            auto *nameLabel = new QLabel(name);
            nameLabel->setMinimumWidth(180);
            rowLayout->addWidget(nameLabel);

            auto *sizeLabelWidget = new QLabel(sizeLabel);
            sizeLabelWidget->setMinimumWidth(70);
            sizeLabelWidget->setStyleSheet("color: gray; font-size: 11px;");
            rowLayout->addWidget(sizeLabelWidget);

            rowLayout->addStretch();

            if (!downloaded) {
                auto *dlBtn = new QPushButton("Download");
                dlBtn->setFixedHeight(24);
                rowLayout->addWidget(dlBtn);

                auto *progBar = new QProgressBar();
                progBar->setMaximumWidth(140);
                progBar->setFixedHeight(18);
                progBar->setMaximum(100);
                progBar->setValue(0);
                progBar->setFormat(QString());
                progBar->setVisible(false);
                rowLayout->addWidget(progBar);

                auto *cancelBtn = new QPushButton("✕");
                cancelBtn->setFixedSize(24, 24);
                cancelBtn->setToolTip("Cancel download");
                cancelBtn->setVisible(false);
                cancelBtn->setStyleSheet("color: red;");
                rowLayout->addWidget(cancelBtn);

                connect(dlBtn, &QPushButton::clicked, this,
                        [=, this, &downloadIds, &downloadBars, &refreshModelList]() {
                    dlBtn->setVisible(false);
                    progBar->setVisible(true);
                    cancelBtn->setVisible(true);
                    progBar->setFormat("0%");

                    QDir().mkpath(QFileInfo(path).absolutePath());
                    int dlId = m_modelManager ? m_modelManager->downloadModel(url, path) : -1;
                    if (dlId >= 0) {
                        downloadIds[url] = dlId;
                        downloadBars[dlId] = progBar;
                    }

                    connect(cancelBtn, &QPushButton::clicked, this,
                            [=, this, &downloadIds, &downloadBars, &refreshModelList]() {
                        if (downloadIds.contains(url)) {
                            int id = downloadIds[url];
                            if (m_modelManager)
                                m_modelManager->cancelDownload(id);
                            downloadIds.remove(url);
                            downloadBars.remove(id);
                            refreshModelList();
                        }
                    });

                    // Periodic refresh to detect download completion
                    QTimer::singleShot(3000, this, [=, &refreshModelList]() {
                        refreshModelList();
                    });
                });
            } else {
                if (!isActive) {
                    auto *selBtn = new QPushButton("Select");
                    selBtn->setFixedHeight(24);
                    rowLayout->addWidget(selBtn);
                    connect(selBtn, &QPushButton::clicked, this, [=, this]() {
                        settings->setValue("ai/local_model_path", path);
                        settings->sync();
                        refreshModelList();
                    });
                } else {
                    auto *activeLabel = new QLabel("Active");
                    activeLabel->setStyleSheet("color: green; font-weight: bold; font-size: 11px;");
                    activeLabel->setMinimumWidth(50);
                    activeLabel->setAlignment(Qt::AlignCenter);
                    rowLayout->addWidget(activeLabel);
                }

                auto *delBtn = new QPushButton("Delete");
                delBtn->setFixedHeight(24);
                delBtn->setStyleSheet("color: red;");
                rowLayout->addWidget(delBtn);
                connect(delBtn, &QPushButton::clicked, this, [=, this]() {
                    auto answer = QMessageBox::question(
                        dlg,
                        "Delete Model",
                        QString("Delete \"%1\"?\n\nApproximately %2 of disk space will be freed.")
                            .arg(name, sizeLabel),
                        QMessageBox::Yes | QMessageBox::No,
                        QMessageBox::No);
                    if (answer == QMessageBox::Yes) {
                        if (m_modelManager)
                            m_modelManager->deleteModel(path);
                        if (settings->value("ai/local_model_path").toString() == path) {
                            settings->remove("ai/local_model_path");
                            settings->sync();
                        }
                        refreshModelList();
                    }
                });
            }

            modelListLayout->addWidget(row);
        }

        modelListLayout->addStretch();
    };

    refreshModelList();

    modelScroll->setWidget(modelListWidget);
    aiLayout->addWidget(modelScroll);

    auto *gpuAccelCheck = new QCheckBox("Enable GPU acceleration");
    gpuAccelCheck->setChecked(settings->value("ai/gpu_acceleration", true).toBool());
    gpuAccelCheck->setToolTip("Offload model layers to GPU for faster inference. Disable if you encounter crashes or have limited VRAM.");
    aiLayout->addWidget(gpuAccelCheck);

    // Save on accept
    connect(&dialog, &QDialog::accepted, this, [settings = settings.get(), apiKeyInput, themeCombo,
            gitNameInput, gitEmailInput, aiModelCombo, aiPromptInput, aiDescriptionPromptInput, aiEnableCheck,
            aiProviderCombo, projectsPathInput, settingsPathInput, themesPathInput,
            gpuAccelCheck, this]() {
        settings->setValue("paths/projects", projectsPathInput->text().trimmed());
        settings->setValue("paths/settings", settingsPathInput->text().trimmed());
        settings->setValue("paths/themes", themesPathInput->text().trimmed());
        settings->sync();
        settings->setValue("ai/enabled", aiEnableCheck->isChecked());
        settings->setValue("ai/provider", aiProviderCombo->currentText());
        settings->setValue("ai/api_key", apiKeyInput->text());
        QString modelText = aiModelCombo->currentText().trimmed();
        if (modelText.isEmpty()) {
            QString provider = aiProviderCombo->currentText();
            if (provider == "OpenRouter")
                modelText = "deepseek/deepseek-v4-flash";
        }
        settings->setValue("ai/model", modelText);

        auto *app = qobject_cast<QApplication *>(qApp);
        QString themeText = themeCombo->currentText();
        if (themeText == "System Default") {
            settings->setValue("appearance/theme", "system");
            if (app) app->setStyleSheet({});
        } else if (themeText == "Dark") {
            settings->setValue("appearance/theme", "dark");
            if (app)
                app->setStyleSheet("QWidget { background-color: #1e1e1e; color: #d4d4d4; }"
                                   "QTreeWidget, QListWidget { background-color: #252526; }"
                                   "QPushButton { background-color: #0e639c; color: white; }"
                                   "QLineEdit, QTextEdit { background-color: #3c3c3c; color: #d4d4d4; }"
                                   "QToolTip { background-color: #3c3c3c; color: #d4d4d4; }");
        } else {
            settings->setValue("appearance/theme", themeText);
            if (app) {
                Theme t = findTheme(themeText, loadCustomThemes());
                if (!t.name.isEmpty())
                    app->setStyleSheet(generateStylesheet(t));
                else
                    app->setStyleSheet({});
            }
        }

        settings->setValue("ai/system_prompt", aiPromptInput->toPlainText().trimmed());
        settings->setValue("ai/description_system_prompt", aiDescriptionPromptInput->toPlainText().trimmed());
        settings->setValue("ai/gpu_acceleration", gpuAccelCheck->isChecked());

        const QString name = gitNameInput->text().trimmed();
        const QString email = gitEmailInput->text().trimmed();
        if (!name.isEmpty())
            runGitConfigSet("user.name", name);
        if (!email.isEmpty())
            runGitConfigSet("user.email", email);

        // Show/hide AI row based on enabled state and provider
        bool aiEnabled = aiEnableCheck->isChecked();
        bool isLocal = aiProviderCombo->currentText() == "Local (internal llama.cpp)";
        bool hasKey = !apiKeyInput->text().trimmed().isEmpty();
        if (aiEnabled && (hasKey || isLocal)) {
            if (m_aiRowContainer && !m_aiRowContainer->isVisible())
                m_aiRowContainer->show();
        } else {
            m_aiRowContainer->hide();
        }
    });

    stack->addWidget(generalPage);
    stack->addWidget(appearancePage);
    stack->addWidget(gitPage);
    stack->addWidget(aiPage);

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

    // Cancel any active model downloads when dialog closes
    connect(&dialog, &QDialog::finished, this, [&downloadIds, this]() {
        for (auto it = downloadIds.constBegin(); it != downloadIds.constEnd(); ++it)
            m_modelManager->cancelDownload(it.value());
        downloadIds.clear();
    });

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

void MainWindow::addGitFileToTree(const QString &path, const QString &prefix, QTreeWidgetItem *parent)
{
    Q_UNUSED(parent);
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

    auto *checkAct = menu.addAction(
        item->checkState(0) == Qt::Checked ? "Uncheck" : "Check");
    connect(checkAct, &QAction::triggered, this, [this, item]() {
        item->setCheckState(0,
            item->checkState(0) == Qt::Checked ? Qt::Unchecked : Qt::Checked);
    });

    menu.addSeparator();

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

    // Check if it's a binary (image/video) file by extension
    static const QStringList imgExts{"png","jpg","jpeg","gif","bmp","webp","svg","ico","tiff","tif"};
    static const QStringList vidExts{"mp4","webm","avi","mov","mkv","wmv","flv"};
    QString ext = QFileInfo(relPath).suffix().toLower();
    bool isImg = imgExts.contains(ext);
    bool isVid = vidExts.contains(ext);

    if (isImg || isVid) {
        if (isImg) {
            QPixmap pm(m_repoPath + "/" + relPath);
            if (pm.isNull()) {
                m_binaryPreview->setText("Could not load image:\n" + relPath);
            } else {
                // Scale down if larger than 800x600 while keeping aspect ratio
                m_binaryPreview->setPixmap(pm.scaled(800, 600, Qt::KeepAspectRatio, Qt::SmoothTransformation));
            }
        } else {
            m_binaryPreview->setText("Video file:\n" + relPath + "\n\nOpen externally to view.");
        }
        m_viewerStack->setCurrentIndex(2);
        return;
    }

    m_viewerStack->setCurrentIndex(0);

    // Block watcher signals so git reads don't trigger a status refresh
    // that would reset checkbox states
    m_fsWatcher->blockSignals(true);

    QString diff = runGitDiff(m_repoPath, {"diff", "HEAD", "--", relPath});

    if (diff.isEmpty())
        diff = runGitDiff(m_repoPath, {"diff", "@{u}..HEAD", "--", relPath});

    m_fsWatcher->blockSignals(false);

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
    QStringList files = checkedFiles();

    if (files.isEmpty()) {
        QMessageBox::information(this, "Nothing Selected",
            "Check at least one file to commit.");
        return;
    }

    // Stage checked files first
    QProcess stageProc;
    stageProc.setWorkingDirectory(m_repoPath);
    stageProc.start("git", QStringList({"add", "--"}) + files);
    if (!stageProc.waitForFinished(5000) || stageProc.exitCode() != 0) {
        QMessageBox::warning(this, "Staging Failed",
            QString::fromUtf8(stageProc.readAllStandardError()));
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

    m_commitButton->setEnabled(false);
    m_commitButton->setText("Committing…");

    QStringList args = {"commit"};
    if (m_skipHooksButton && m_skipHooksButton->isChecked())
        args << "--no-verify";
    args << "-m" << m_summaryInput->text().trimmed();

    const QString desc = m_descriptionInput->toPlainText().trimmed();
    if (!desc.isEmpty())
        args << "-m" << desc;

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

void MainWindow::onStageAllFiles()
{
    setAllCheckStates(Qt::Checked);
}

void MainWindow::onUnstageAllFiles()
{
    setAllCheckStates(Qt::Unchecked);
}

void MainWindow::readDiffForFile(const QString &file) const
{
    Q_UNUSED(file);
}

void MainWindow::onEnableAiSystem()
{
    // AI system is always enabled when configured
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
        updateStagedUnstagedTrees(output);
        startGitUnpushedQuery();
    } else if (m_currentQuery == GitQuery::Unpushed) {
        const QStringList lines = output.split('\n', Qt::SkipEmptyParts);

        for (const QString &line : lines) {
            const QString trimmed = line.trimmed();
            if (trimmed.isEmpty())
                continue;

            addGitFileToTree(trimmed, "[P]");
        }

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

void MainWindow::updateStagedUnstagedTrees(const QString &output)
{
    // Save existing check states before clearing
    QSet<QString> uncheckedFiles;
    for (int i = 0; i < m_gitStatusTree->topLevelItemCount(); ++i) {
        auto *item = m_gitStatusTree->topLevelItem(i);
        if (item->checkState(0) == Qt::Unchecked)
            uncheckedFiles.insert(item->data(0, Qt::UserRole).toString());
    }

    m_gitStatusTree->clear();

    const QStringList lines = output.split('\n', Qt::SkipEmptyParts);
    int count = 0;

    for (const QString &line : lines) {
        const QString xy = line.left(2);

        QString prefix;
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

        auto *fileItem = new QTreeWidgetItem();
        fileItem->setText(0, path);
        fileItem->setData(0, Qt::UserRole, path);
        fileItem->setFlags(fileItem->flags() | Qt::ItemIsUserCheckable);
        fileItem->setCheckState(0, uncheckedFiles.contains(path) ? Qt::Unchecked : Qt::Checked);

        const QChar status = prefix.trimmed().isEmpty() ? QChar() : prefix.trimmed().at(1);
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
        ++count;
    }

    m_changedFilesLabel->setText(QString("%1 files").arg(count));

    // Update select-all checkbox: checked if all checked, unchecked if any unchecked, partial otherwise
    if (count == 0)
        m_selectAllCheck->setCheckState(Qt::Checked);
    else if (uncheckedFiles.isEmpty())
        m_selectAllCheck->setCheckState(Qt::Checked);
    else if (uncheckedFiles.size() == count)
        m_selectAllCheck->setCheckState(Qt::Unchecked);
    else
        m_selectAllCheck->setCheckState(Qt::PartiallyChecked);
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
    if (m_commitProcess && m_commitProcess->state() != QProcess::NotRunning)
        return;
    m_refreshTimer->start();
}

void MainWindow::onRefreshDebounce()
{
    if (m_repoPath.isEmpty())
        return;
    if (m_commitProcess && m_commitProcess->state() != QProcess::NotRunning)
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

#ifdef Q_OS_WIN
    // Try winget first, then chocolatey
    QStringList args;
    if (!QStandardPaths::findExecutable("winget").isEmpty()) {
        args = {"install", "--id", "Git.Git", "-e", "--source", "winget"};
        m_installProcess->start("winget", args);
    } else if (!QStandardPaths::findExecutable("choco").isEmpty()) {
        args = {"install", "git", "-y"};
        m_installProcess->start("choco", args);
    } else {
        QDesktopServices::openUrl(QUrl("https://git-scm.com/downloads/win"));
    }
#elif defined(Q_OS_MACOS)
    // Try Homebrew first, then MacPorts, then Xcode CLI tools
    if (!QStandardPaths::findExecutable("brew").isEmpty()) {
        m_installProcess->start("brew", {"install", "git"});
    } else if (!QStandardPaths::findExecutable("port").isEmpty()) {
        m_installProcess->start("port", {"install", "git"});
    } else {
        QDesktopServices::openUrl(QUrl("https://git-scm.com/downloads/mac"));
    }
#else
    // Linux — open the downloads page; package managers vary too much
    // and pkexec/sudo prompts are disruptive in a GUI app.
    QDesktopServices::openUrl(QUrl("https://git-scm.com/downloads/linux"));
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

void MainWindow::onLocalAiResponse(const QString &text)
{
    if (m_aiThinkingOverlay)
        m_aiThinkingOverlay->setVisible(false);

    m_aiCommitButton->setEnabled(true);
    m_aiDescriptionButton->setEnabled(true);
    m_summaryInput->setEnabled(true);
    m_descriptionInput->setEnabled(true);

    if (text.isEmpty())
        return;

    if (m_aiRequestKind == AiRequestKind::Description) {
        m_descriptionInput->setPlainText(text.trimmed());
        return;
    }

    QStringList lines = text.split('\n', Qt::SkipEmptyParts);
    if (lines.isEmpty())
        return;

    QString summary = lines.first();
    summary.remove(QRegularExpression("^#+\\s*"));
    summary.remove(QRegularExpression("^commit:\\s*", QRegularExpression::CaseInsensitiveOption));
    summary = summary.trimmed();
    if (!summary.isEmpty())
        m_summaryInput->setText(summary);
}

void MainWindow::onLocalAiError(const QString &error)
{
    if (m_aiThinkingOverlay)
        m_aiThinkingOverlay->setVisible(false);

    m_aiCommitButton->setEnabled(true);
    m_aiDescriptionButton->setEnabled(true);
    m_summaryInput->setEnabled(true);
    m_descriptionInput->setEnabled(true);

    QMessageBox::warning(this, "Local AI Error", error);
}

void MainWindow::onLocalAiThinking(const QString &token)
{
    if (m_aiThinkingText) {
        m_aiThinkingText->moveCursor(QTextCursor::End);
        m_aiThinkingText->insertPlainText(token);
        m_aiThinkingText->moveCursor(QTextCursor::End);
    }
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
