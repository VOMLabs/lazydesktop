#include "vcs_bridge.h"
#include "vcs_core.h"

#include <QDebug>
#include <QJsonDocument>
#include <QJsonObject>

namespace
{

    // Runs a vcs_* call that returns a heap-allocated UTF-8 string, hands it to
    // Qt, and frees it via the crate's vcs_free_string.
    QString takeResult(char* result)
    {
        if (!result)
            return {};
        const QString text = QString::fromUtf8(result);
        vcs_free_string(result);
        return text;
    }

} // namespace

VcsBridge::VcsBridge(QObject* parent) : QObject(parent)
{
}

// ─── SSH keys ──────────────────────────────────────────

QStringList VcsBridge::listSshPublicKeys()
{
    const QString json = takeResult(vcs_ssh_list_public_keys());
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    if (!doc.isArray())
        return {};

    QStringList keys;
    const QJsonArray arr = doc.array();
    keys.reserve(arr.size());
    for (const QJsonValue& value : arr)
        keys << value.toString();
    return keys;
}

QString VcsBridge::readPublicKey(const QString& pubPath)
{
    return takeResult(vcs_ssh_read_public_key(pubPath.toUtf8().constData()));
}

QString VcsBridge::fingerprint(const QString& pubPath)
{
    return takeResult(vcs_ssh_fingerprint(pubPath.toUtf8().constData()));
}

QString VcsBridge::generateKey(const QString& type, const QString& path, const QString& comment,
                               const QString& passphrase)
{
    const QByteArray typeUtf8 = type.toUtf8();
    const QByteArray pathUtf8 = path.toUtf8();
    const QByteArray commentUtf8 = comment.toUtf8();
    const QByteArray passUtf8 = passphrase.toUtf8();

    return takeResult(vcs_ssh_generate_key(typeUtf8.constData(), pathUtf8.constData(), commentUtf8.constData(),
                                           passphrase.isEmpty() ? nullptr : passUtf8.constData()));
}

QString VcsBridge::testConnection(const QString& host, int port)
{
    const QByteArray hostUtf8 = host.toUtf8();
    return takeResult(vcs_ssh_test_connection(hostUtf8.constData(), static_cast<uint16_t>(port)));
}

// ─── Git remotes ───────────────────────────────────────

QJsonArray VcsBridge::listRemotes(const QString& repoPath)
{
    const QString json = takeResult(vcs_remote_list(repoPath.toUtf8().constData()));
    const QJsonDocument doc = QJsonDocument::fromJson(json.toUtf8());
    return doc.isArray() ? doc.array() : QJsonArray();
}

QString VcsBridge::addRemote(const QString& repoPath, const QString& name, const QString& url)
{
    const QByteArray pathUtf8 = repoPath.toUtf8();
    const QByteArray nameUtf8 = name.toUtf8();
    const QByteArray urlUtf8 = url.toUtf8();
    return takeResult(vcs_remote_add(pathUtf8.constData(), nameUtf8.constData(), urlUtf8.constData()));
}

QString VcsBridge::removeRemote(const QString& repoPath, const QString& name)
{
    const QByteArray pathUtf8 = repoPath.toUtf8();
    const QByteArray nameUtf8 = name.toUtf8();
    return takeResult(vcs_remote_remove(pathUtf8.constData(), nameUtf8.constData()));
}

QString VcsBridge::setRemoteUrl(const QString& repoPath, const QString& name, const QString& url)
{
    const QByteArray pathUtf8 = repoPath.toUtf8();
    const QByteArray nameUtf8 = name.toUtf8();
    const QByteArray urlUtf8 = url.toUtf8();
    return takeResult(vcs_remote_set_url(pathUtf8.constData(), nameUtf8.constData(), urlUtf8.constData()));
}

QString VcsBridge::renameRemote(const QString& repoPath, const QString& oldName, const QString& newName)
{
    const QByteArray pathUtf8 = repoPath.toUtf8();
    const QByteArray oldUtf8 = oldName.toUtf8();
    const QByteArray newUtf8 = newName.toUtf8();
    return takeResult(vcs_remote_rename(pathUtf8.constData(), oldUtf8.constData(), newUtf8.constData()));
}
