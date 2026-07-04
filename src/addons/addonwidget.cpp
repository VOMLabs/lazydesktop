#include "addonwidget.h"
#include "addonmanager.h"
#include "addonmanifest.h"
#include "addonruntime.h"
#include "addonpermissions.h"
#include "addoninstaller.h"

#include <QCloseEvent>

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QSplitter>
#include <QLabel>
#include <QPushButton>
#include <QListWidget>
#include <QStackedWidget>
#include <QLineEdit>
#include <QFileDialog>
#include <QMessageBox>
#include <QDesktopServices>
#include <QUrl>
#include <QGroupBox>
#include <QFrame>
#include <QStyle>
#include <QDir>

AddonWidget::AddonWidget(QWidget *parent)
    : QDialog(parent)
{
    m_manager = AddonManager::instance();
    setWindowTitle("LazyAddons Manager");
    setMinimumSize(750, 500);
    resize(800, 550);

    setupUi();

    connect(m_manager, &AddonManager::addonsChanged, this, &AddonWidget::refreshList);
    refreshList();

    m_manager->dispatchEventToAll(AddonEvent::WindowCreated, {{"widget", "manager"}});
}

void AddonWidget::closeEvent(QCloseEvent *event)
{
    m_manager->dispatchEventToAll(AddonEvent::WindowClosed, {{"widget", "manager"}});
    QDialog::closeEvent(event);
}

void AddonWidget::setupUi()
{
    auto *mainLayout = new QVBoxLayout(this);
    mainLayout->setContentsMargins(8, 8, 8, 8);
    mainLayout->setSpacing(6);

    auto *headerLayout = new QHBoxLayout();
    auto *headerLabel = new QLabel("<b>LazyAddons</b>");
    headerLabel->setStyleSheet("font-size: 16px;");
    headerLayout->addWidget(headerLabel);
    headerLayout->addStretch();

    m_installBtn = new QPushButton("Install Addon...");
    m_installBtn->setMinimumHeight(28);
    connect(m_installBtn, &QPushButton::clicked, this, &AddonWidget::onInstallClicked);
    headerLayout->addWidget(m_installBtn);

    mainLayout->addLayout(headerLayout);

    m_searchInput = new QLineEdit();
    m_searchInput->setPlaceholderText("Search installed addons...");
    m_searchInput->setClearButtonEnabled(true);
    connect(m_searchInput, &QLineEdit::textChanged, this, &AddonWidget::onSearchChanged);
    mainLayout->addWidget(m_searchInput);

    auto *splitter = new QSplitter(Qt::Horizontal);

    auto *leftPanel = new QWidget();
    auto *leftLayout = new QVBoxLayout(leftPanel);
    leftLayout->setContentsMargins(0, 0, 0, 0);

    m_pluginList = new QListWidget();
    m_pluginList->setAlternatingRowColors(true);
    m_pluginList->setMinimumWidth(220);
    connect(m_pluginList, &QListWidget::currentRowChanged, this, [this](int) {
        onPluginSelected();
    });
    leftLayout->addWidget(m_pluginList);

    auto *listActions = new QHBoxLayout();

    m_toggleBtn = new QPushButton("Enable");
    m_toggleBtn->setEnabled(false);
    m_toggleBtn->setMinimumHeight(26);
    connect(m_toggleBtn, &QPushButton::clicked, this, &AddonWidget::onToggleClicked);
    listActions->addWidget(m_toggleBtn);

    m_reloadBtn = new QPushButton("Reload");
    m_reloadBtn->setEnabled(false);
    m_reloadBtn->setMinimumHeight(26);
    connect(m_reloadBtn, &QPushButton::clicked, this, &AddonWidget::onReloadClicked);
    listActions->addWidget(m_reloadBtn);

    leftLayout->addLayout(listActions);

    splitter->addWidget(leftPanel);

    auto *rightPanel = new QWidget();
    auto *rightLayout = new QVBoxLayout(rightPanel);
    rightLayout->setContentsMargins(8, 0, 0, 0);

    m_detailsStack = new QStackedWidget();

    m_emptyPage = new QWidget();
    auto *emptyLayout = new QVBoxLayout(m_emptyPage);
    emptyLayout->addStretch();
    auto *emptyLabel = new QLabel("Select an addon to view details");
    emptyLabel->setAlignment(Qt::AlignCenter);
    emptyLabel->setStyleSheet("color: gray; font-size: 14px;");
    emptyLayout->addWidget(emptyLabel);
    emptyLayout->addStretch();

    m_detailsPage = new QWidget();
    auto *detailsLayout = new QVBoxLayout(m_detailsPage);
    detailsLayout->setSpacing(6);

    auto *headerGroup = new QGroupBox();
    auto *headerGroupLayout = new QVBoxLayout(headerGroup);

    m_nameLabel = new QLabel();
    m_nameLabel->setStyleSheet("font-size: 18px; font-weight: bold;");
    headerGroupLayout->addWidget(m_nameLabel);

    m_idLabel = new QLabel();
    m_idLabel->setStyleSheet("color: gray; font-family: monospace;");
    headerGroupLayout->addWidget(m_idLabel);

    detailsLayout->addWidget(headerGroup);

    auto *metaGroup = new QGroupBox("Metadata");
    auto *metaLayout = new QVBoxLayout(metaGroup);

    auto addMetaRow = [&](const QString &label, QLabel *&valueLabel) {
        auto *row = new QHBoxLayout();
        auto *lbl = new QLabel(label);
        lbl->setStyleSheet("color: gray;");
        lbl->setFixedWidth(80);
        valueLabel = new QLabel();
        valueLabel->setWordWrap(true);
        valueLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
        row->addWidget(lbl);
        row->addWidget(valueLabel, 1);
        metaLayout->addLayout(row);
    };

    addMetaRow("Version:", m_versionLabel);
    addMetaRow("Author:", m_authorLabel);
    addMetaRow("Runtime:", m_runtimeLabel);
    addMetaRow("Status:", m_statusLabel);
    addMetaRow("Entry:", m_entryLabel);
    addMetaRow("Size:", m_sizeLabel);
    addMetaRow("Perms:", m_permsLabel);

    detailsLayout->addWidget(metaGroup);

    auto *descGroup = new QGroupBox("Description");
    auto *descLayout = new QVBoxLayout(descGroup);
    m_descLabel = new QLabel();
    m_descLabel->setWordWrap(true);
    descLayout->addWidget(m_descLabel);
    detailsLayout->addWidget(descGroup);

    detailsLayout->addStretch();

    auto *detailsActions = new QHBoxLayout();

    m_uninstallBtn = new QPushButton("Uninstall");
    m_uninstallBtn->setEnabled(false);
    m_uninstallBtn->setStyleSheet("QPushButton { color: #e04040; }");
    connect(m_uninstallBtn, &QPushButton::clicked, this, &AddonWidget::onUninstallClicked);
    detailsActions->addWidget(m_uninstallBtn);

    m_openDirBtn = new QPushButton("Open Plugin Directory");
    m_openDirBtn->setEnabled(false);
    connect(m_openDirBtn, &QPushButton::clicked, this, &AddonWidget::onOpenDirClicked);
    detailsActions->addWidget(m_openDirBtn);

    detailsActions->addStretch();
    detailsLayout->addLayout(detailsActions);

    m_detailsStack->addWidget(m_emptyPage);
    m_detailsStack->addWidget(m_detailsPage);
    m_detailsStack->setCurrentWidget(m_emptyPage);

    rightLayout->addWidget(m_detailsStack, 1);

    splitter->addWidget(rightPanel);
    splitter->setStretchFactor(0, 1);
    splitter->setStretchFactor(1, 2);

    mainLayout->addWidget(splitter, 1);

    auto *bottomBar = new QHBoxLayout();
    bottomBar->addStretch();
    auto *closeBtn = new QPushButton("Close");
    closeBtn->setMinimumHeight(28);
    connect(closeBtn, &QPushButton::clicked, this, &QDialog::accept);
    bottomBar->addWidget(closeBtn);
    mainLayout->addLayout(bottomBar);
}

void AddonWidget::onPluginSelected()
{
    auto *item = m_pluginList->currentItem();
    if (!item) {
        clearDetails();
        return;
    }

    QString pluginId = item->data(Qt::UserRole).toString();
    m_selectedPluginId = pluginId;
    showPluginDetails(pluginId);
}

void AddonWidget::onInstallClicked()
{
    QString path = QFileDialog::getOpenFileName(
        this, "Install Addon", {},
        "LazyAddons Packages (*.lza *.zip);;All Files (*)");

    if (path.isEmpty()) return;

    QStringList perms;
    AddonInstaller installer;
    auto result = installer.install(path, m_manager->addonsDir());

    if (result.success) {
        perms = result.manifest.permissions;
        if (!perms.isEmpty()) {
            showPermissionWarning(perms);
        }

        QDir(result.extractPath).removeRecursively();
    }

    if (m_manager->installFromArchive(path)) {
        QMessageBox::information(this, "Addon Installed",
                                 "The addon was installed successfully.");
    } else {
        QMessageBox::warning(this, "Install Failed",
                             "Failed to install addon. Check that the package is valid.");
    }
}

void AddonWidget::onUninstallClicked()
{
    if (m_selectedPluginId.isEmpty()) return;

    auto result = QMessageBox::question(
        this, "Uninstall Addon",
        "Are you sure you want to uninstall \"" + m_selectedPluginId + "\"?",
        QMessageBox::Yes | QMessageBox::No);

    if (result == QMessageBox::Yes) {
        m_manager->uninstall(m_selectedPluginId);
        m_selectedPluginId.clear();
        clearDetails();
    }
}

void AddonWidget::onToggleClicked()
{
    if (m_selectedPluginId.isEmpty()) return;

    if (m_manager->isEnabled(m_selectedPluginId))
        m_manager->disable(m_selectedPluginId);
    else
        m_manager->enable(m_selectedPluginId);

    showPluginDetails(m_selectedPluginId);
    refreshList();
}

void AddonWidget::onReloadClicked()
{
    if (m_selectedPluginId.isEmpty()) return;
    m_manager->reload(m_selectedPluginId);
    showPluginDetails(m_selectedPluginId);
}

void AddonWidget::onOpenDirClicked()
{
    if (m_selectedPluginId.isEmpty()) return;
    QString dir = m_manager->pluginDir(m_selectedPluginId);
    QDesktopServices::openUrl(QUrl::fromLocalFile(dir));
}

void AddonWidget::onSearchChanged(const QString &text)
{
    for (int i = 0; i < m_pluginList->count(); i++) {
        auto *item = m_pluginList->item(i);
        bool match = text.isEmpty() ||
                     item->text().contains(text, Qt::CaseInsensitive) ||
                     item->data(Qt::UserRole).toString().contains(text, Qt::CaseInsensitive);
        item->setHidden(!match);
    }
}

void AddonWidget::refreshList()
{
    QString currentId = m_selectedPluginId;
    m_pluginList->clear();

    for (const auto &id : m_manager->installedPlugins()) {
        auto *rt = m_manager->runtime(id);
        QString display = rt ? rt->manifest().name : id;
        if (display.isEmpty()) display = id;

        auto *item = new QListWidgetItem(display);
        item->setData(Qt::UserRole, id);

        if (rt) {
            if (rt->status() == AddonStatus::Error || rt->status() == AddonStatus::Incompatible)
                item->setForeground(QColor("#e04040"));
            else if (rt->isEnabled())
                item->setForeground(QColor("#4ec9b0"));
            else
                item->setForeground(QColor("#888888"));
        }

        m_pluginList->addItem(item);
    }

    m_pluginList->sortItems();

    for (int i = 0; i < m_pluginList->count(); i++) {
        if (m_pluginList->item(i)->data(Qt::UserRole).toString() == currentId) {
            m_pluginList->setCurrentRow(i);
            break;
        }
    }
}

void AddonWidget::showPluginDetails(const QString &pluginId)
{
    auto *rt = m_manager->runtime(pluginId);
    if (!rt) {
        clearDetails();
        return;
    }

    auto manifest = rt->manifest();

    m_nameLabel->setText(manifest.name.isEmpty() ? pluginId : manifest.name);
    m_idLabel->setText(pluginId);
    m_versionLabel->setText(manifest.version);
    m_authorLabel->setText(manifest.author.isEmpty() ? "Unknown" : manifest.author);
    m_runtimeLabel->setText(manifest.runtime);
    m_statusLabel->setText(rt->statusString());
    m_entryLabel->setText(manifest.entry);
    m_descLabel->setText(manifest.description.isEmpty() ? "No description" : manifest.description);

    qint64 size = m_manager->pluginSize(pluginId);
    if (size < 1024)
        m_sizeLabel->setText(QString::number(size) + " B");
    else if (size < 1024 * 1024)
        m_sizeLabel->setText(QString::number(size / 1024) + " KB");
    else
        m_sizeLabel->setText(QString::number(size / (1024 * 1024)) + " MB");

    if (manifest.permissions.isEmpty()) {
        m_permsLabel->setText("None");
    } else {
        QStringList perms = AddonPermissions::dangerousPermissions(
            AddonPermissions::fromStringList(manifest.permissions));
        if (perms.isEmpty())
            m_permsLabel->setText(manifest.permissions.join(", "));
        else
            m_permsLabel->setText(manifest.permissions.join(", ") +
                                  " <span style='color:#e04040;'>(dangerous: " +
                                  perms.join(", ") + ")</span>");
        m_permsLabel->setTextFormat(Qt::RichText);
    }

    m_toggleBtn->setEnabled(true);
    m_toggleBtn->setText(rt->isEnabled() ? "Disable" : "Enable");

    m_reloadBtn->setEnabled(true);
    m_uninstallBtn->setEnabled(true);
    m_openDirBtn->setEnabled(true);

    m_detailsStack->setCurrentWidget(m_detailsPage);
}

void AddonWidget::showPermissionWarning(const QStringList &permissions) const
{
    if (permissions.isEmpty()) return;

    auto perms = AddonPermissions::fromStringList(permissions);
    QStringList dangerous = AddonPermissions::dangerousPermissions(perms);

    if (dangerous.isEmpty()) return;

    QString msg = "This addon requests the following sensitive permissions:\n\n";
    for (const auto &p : dangerous) {
        msg += "  \u2022 " + p + "\n";
        msg += "      " + AddonPermissions::description(AddonPermissions::fromString(p)) + "\n";
    }
    msg += "\nInstall anyway?";

    QMessageBox::warning(const_cast<AddonWidget *>(this), "Permission Warning", msg);
}

void AddonWidget::clearDetails()
{
    m_nameLabel->clear();
    m_idLabel->clear();
    m_versionLabel->clear();
    m_authorLabel->clear();
    m_descLabel->clear();
    m_runtimeLabel->clear();
    m_statusLabel->clear();
    m_entryLabel->clear();
    m_sizeLabel->clear();
    m_permsLabel->clear();

    m_toggleBtn->setEnabled(false);
    m_reloadBtn->setEnabled(false);
    m_uninstallBtn->setEnabled(false);
    m_openDirBtn->setEnabled(false);

    m_detailsStack->setCurrentWidget(m_emptyPage);
}
