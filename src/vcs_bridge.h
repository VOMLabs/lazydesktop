#ifndef VCS_BRIDGE_H
#define VCS_BRIDGE_H

#include <QJsonArray>
#include <QObject>
#include <QString>
#include <QStringList>

// VcsBridge wraps the native vcs_core FFI (crates/vcs_core/vcs_core.h) for
// use from Qt code. All methods are synchronous; connection tests are bounded
// by a 10s timeout inside the crate. The bridge owns the returned strings and
// frees them via vcs_free_string. Private key material never crosses into Qt.
class VcsBridge : public QObject
{
    Q_OBJECT

public:
    explicit VcsBridge(QObject* parent = nullptr);

    // SSH keys -----------------------------------------------------------
    // Absolute paths of every *.pub key in ~/.ssh, sorted by name.
    QStringList listSshPublicKeys();
    // Trimmed OpenSSH text of a public key file ("" on error).
    QString readPublicKey(const QString& pubPath);
    // SHA256 fingerprint of a public key file ("" on error).
    QString fingerprint(const QString& pubPath);
    // Generates a keypair. Returns an empty string on success, otherwise an
    // error message. The private key stays on disk.
    QString generateKey(const QString& type, const QString& path, const QString& comment, const QString& passphrase);
    // Tests an SSH connection in batch mode (no prompts, 10s timeout).
    // Returns a human-readable result or error message.
    QString testConnection(const QString& host, int port);

    // Git remotes --------------------------------------------------------
    // JSON array of {"name": ..., "url": ...} objects (empty on error).
    QJsonArray listRemotes(const QString& repoPath);
    // Each returns "" on success, otherwise an error message.
    QString addRemote(const QString& repoPath, const QString& name, const QString& url);
    QString removeRemote(const QString& repoPath, const QString& name);
    QString setRemoteUrl(const QString& repoPath, const QString& name, const QString& url);
    QString renameRemote(const QString& repoPath, const QString& oldName, const QString& newName);
};

#endif // VCS_BRIDGE_H
