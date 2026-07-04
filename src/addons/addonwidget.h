#ifndef ADDONWIDGET_H
#define ADDONWIDGET_H

#include <QDialog>
#include <QHash>

class QListWidget;
class QListWidgetItem;
class QStackedWidget;
class QLabel;
class QPushButton;
class QLineEdit;
class AddonManager;

class AddonWidget : public QDialog
{
    Q_OBJECT

public:
    explicit AddonWidget(QWidget *parent = nullptr);

protected:
    void closeEvent(QCloseEvent *event) override;

private slots:
    void onPluginSelected();
    void onInstallClicked();
    void onUninstallClicked();
    void onToggleClicked();
    void onReloadClicked();
    void onOpenDirClicked();
    void onSearchChanged(const QString &text);
    void refreshList();

private:
    void setupUi();
    void showPluginDetails(const QString &pluginId);
    void showPermissionWarning(const QStringList &permissions) const;
    void clearDetails();

    QListWidget *m_pluginList = nullptr;
    QStackedWidget *m_detailsStack = nullptr;
    QWidget *m_detailsPage = nullptr;
    QWidget *m_emptyPage = nullptr;

    QLabel *m_nameLabel = nullptr;
    QLabel *m_idLabel = nullptr;
    QLabel *m_versionLabel = nullptr;
    QLabel *m_authorLabel = nullptr;
    QLabel *m_descLabel = nullptr;
    QLabel *m_runtimeLabel = nullptr;
    QLabel *m_statusLabel = nullptr;
    QLabel *m_permsLabel = nullptr;
    QLabel *m_sizeLabel = nullptr;
    QLabel *m_entryLabel = nullptr;

    QPushButton *m_toggleBtn = nullptr;
    QPushButton *m_reloadBtn = nullptr;
    QPushButton *m_uninstallBtn = nullptr;
    QPushButton *m_openDirBtn = nullptr;
    QPushButton *m_installBtn = nullptr;

    QLineEdit *m_searchInput = nullptr;

    QString m_selectedPluginId;
    AddonManager *m_manager = nullptr;
};

#endif
