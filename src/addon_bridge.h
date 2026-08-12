#ifndef ADDON_BRIDGE_H
#define ADDON_BRIDGE_H

#include <QJsonArray>
#include <QJsonObject>
#include <QObject>
#include <QString>

struct lda_registry;

// AddonBridge wraps the native addons FFI (crates/addons/include/addons.h) for
// use from Qt code. All methods are synchronous; the crate owns all validation
// and path security, so Qt only ever consumes validated JSON descriptors and
// bytes. The bridge owns the registry handle and frees returned strings/bytes
// via the crate's lda_free_string / lda_free_bytes.
class AddonBridge : public QObject
{
    Q_OBJECT

public:
    explicit AddonBridge(const QString& addonsDir, QObject* parent = nullptr);
    ~AddonBridge() override;

    // Returns true when the registry was created and configured successfully.
    [[nodiscard]] bool isValid() const;

    // Listing / lookup -----------------------------------------------------
    // JSON array of addon descriptors (empty on error). Each descriptor has:
    // id, name, version, description?, author?, apiVersion, provider,
    // rootKind, state ("installed" | "shadowed" | "conflict" | "error").
    QJsonArray listAddons();
    // {"addon": {...}} on success, {"error": {...}} when not installed.
    QJsonObject getAddon(const QString& id);

    // Install / uninstall --------------------------------------------------
    // sourcePath may be a directory, .zip, or .lzd package. Returns the
    // installed descriptor {"addon": {...}} on success, or {"error": {...}}.
    QJsonObject install(const QString& sourcePath);
    // Returns an empty string on success, otherwise the error message.
    QString uninstall(const QString& id);

    // Resource access (validated bytes only) -------------------------------
    // Reads an addon asset (e.g. "config.toml") as bytes; empty on error.
    QByteArray readAsset(const QString& id, const QString& relPath);
    // Reads the addon's entry script as bytes; empty on error.
    QByteArray entryScript(const QString& id);

    // Extracts the sanitized error message from an {"error": {...}} payload.
    static QString errorMessage(const QJsonObject& json);

private:
    lda_registry* m_registry = nullptr;
};

#endif // ADDON_BRIDGE_H
