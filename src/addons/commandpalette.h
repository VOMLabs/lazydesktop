#ifndef COMMANDPALETTE_H
#define COMMANDPALETTE_H

#include <QDialog>
#include <QStringList>

class QListWidget;
class QLineEdit;

class CommandPalette : public QDialog
{
    Q_OBJECT

public:
    explicit CommandPalette(QWidget *parent = nullptr);

private slots:
    void onSearchChanged(const QString &text);
    void onCommandActivated();

private:
    void setupUi();
    void populateCommands();

    QLineEdit *m_searchInput = nullptr;
    QListWidget *m_commandList = nullptr;
    QStringList m_commandNames;
};

#endif