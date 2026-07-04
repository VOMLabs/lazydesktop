#include "commandpalette.h"
#include "addonmanager.h"

#include <QVBoxLayout>
#include <QHBoxLayout>
#include <QLineEdit>
#include <QListWidget>
#include <QLabel>
#include <QKeyEvent>

CommandPalette::CommandPalette(QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle("Command Palette");
    setMinimumSize(400, 300);
    resize(450, 350);

    setupUi();
    populateCommands();

    m_searchInput->setFocus();
    connect(m_searchInput, &QLineEdit::textChanged, this, &CommandPalette::onSearchChanged);
    connect(m_commandList, &QListWidget::itemActivated, this, &CommandPalette::onCommandActivated);
}

void CommandPalette::setupUi()
{
    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);
    layout->setSpacing(6);

    m_searchInput = new QLineEdit();
    m_searchInput->setPlaceholderText("Type a command name...");
    m_searchInput->setClearButtonEnabled(true);
    layout->addWidget(m_searchInput);

    m_commandList = new QListWidget();
    m_commandList->setAlternatingRowColors(true);
    layout->addWidget(m_commandList, 1);
}

void CommandPalette::populateCommands()
{
    m_commandList->clear();
    auto *mgr = AddonManager::instance();
    if (!mgr) return;

    m_commandNames = mgr->allCommands();
    m_commandNames.sort(Qt::CaseInsensitive);

    for (const auto &name : m_commandNames) {
        auto *item = new QListWidgetItem(name);
        QString owner = mgr->commandOwner(name);
        if (!owner.isEmpty())
            item->setToolTip("Plugin: " + owner);
        m_commandList->addItem(item);
    }
}

void CommandPalette::onSearchChanged(const QString &text)
{
    for (int i = 0; i < m_commandList->count(); i++) {
        auto *item = m_commandList->item(i);
        bool match = text.isEmpty() ||
                     item->text().contains(text, Qt::CaseInsensitive);
        item->setHidden(!match);
    }
}

void CommandPalette::onCommandActivated()
{
    auto *item = m_commandList->currentItem();
    if (!item) return;

    QString cmdName = item->text();
    auto *mgr = AddonManager::instance();
    if (mgr)
        mgr->executeCommand(cmdName);

    accept();
}
