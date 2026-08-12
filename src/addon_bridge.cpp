#include "addon_bridge.h"
#include "addons.h"

#include <QDebug>
#include <QDir>
#include <QJsonDocument>

namespace
{

    // Takes a heap-allocated UTF-8 string from the crate, converts it to Qt,
    // and frees it via the crate's own allocator.
    QString takeResult(char* result)
    {
        if (!result)
            return {};
        const QString text = QString::fromUtf8(result);
        lda_free_string(result);
        return text;
    }

    // Takes a byte buffer from the crate and frees it with lda_free_bytes.
    QByteArray takeBytes(uint8_t* bytes, size_t len)
    {
        if (!bytes)
            return {};
        const QByteArray data(reinterpret_cast<const char*>(bytes), static_cast<int>(len));
        lda_free_bytes(bytes);
        return data;
    }

} // namespace

AddonBridge::AddonBridge(const QString& addonsDir, QObject* parent) : QObject(parent)
{
    QDir().mkpath(addonsDir);

    m_registry = lda_registry_create();
    if (!m_registry)
    {
        qWarning() << "AddonBridge: lda_registry_create failed";
        return;
    }

    const QByteArray dirUtf8 = addonsDir.toUtf8();
    const QByteArray versionUtf8 = QByteArrayLiteral("0.2.0");
    if (lda_registry_set_app_version(m_registry, versionUtf8.constData()) != LDA_OK)
    {
        qWarning() << "AddonBridge: set_app_version failed";
        return;
    }
    // The user root has priority so per-user addons shadow anything a future
    // system root may install.
    if (lda_registry_add_root(m_registry, dirUtf8.constData(), /*user_priority=*/1) != LDA_OK)
    {
        qWarning() << "AddonBridge: add_root failed";
        return;
    }
    if (lda_registry_load(m_registry) != LDA_OK)
    {
        qWarning() << "AddonBridge: load failed";
        return;
    }
}

AddonBridge::~AddonBridge()
{
    if (m_registry)
    {
        lda_registry_destroy(&m_registry);
        m_registry = nullptr;
    }
}

bool AddonBridge::isValid() const
{
    return m_registry != nullptr;
}

// ─── Listing / lookup ──────────────────────────────────

QJsonArray AddonBridge::listAddons()
{
    if (!m_registry)
        return {};

    char* raw = nullptr;
    if (lda_registry_list(m_registry, &raw) != LDA_OK)
        return {};

    const QJsonDocument doc = QJsonDocument::fromJson(takeResult(raw).toUtf8());
    return doc.object().value(QLatin1String("addons")).toArray();
}

QJsonObject AddonBridge::getAddon(const QString& id)
{
    if (!m_registry)
        return {};

    const QByteArray idUtf8 = id.toUtf8();
    char* raw = nullptr;
    if (lda_registry_get(m_registry, idUtf8.constData(), &raw) != LDA_OK)
        return {};

    return QJsonDocument::fromJson(takeResult(raw).toUtf8()).object();
}

// ─── Install / uninstall ───────────────────────────────

QJsonObject AddonBridge::install(const QString& sourcePath)
{
    if (!m_registry)
        return {};

    const QByteArray pathUtf8 = sourcePath.toUtf8();
    char* raw = nullptr;
    if (lda_registry_install(m_registry, pathUtf8.constData(), &raw) != LDA_OK)
    {
        // The crate still writes an error object; surface it if present.
        if (raw)
            return QJsonDocument::fromJson(takeResult(raw).toUtf8()).object();
        return {};
    }

    return QJsonDocument::fromJson(takeResult(raw).toUtf8()).object();
}

QString AddonBridge::uninstall(const QString& id)
{
    if (!m_registry)
        return QStringLiteral("addon system is unavailable");

    const QByteArray idUtf8 = id.toUtf8();
    const lda_result res = lda_registry_uninstall(m_registry, idUtf8.constData());
    if (res == LDA_OK)
        return {};
    return QStringLiteral("failed to uninstall addon (code %1)").arg(static_cast<int>(res));
}

// ─── Resource access ───────────────────────────────────

QByteArray AddonBridge::readAsset(const QString& id, const QString& relPath)
{
    if (!m_registry)
        return {};

    const QByteArray idUtf8 = id.toUtf8();
    const QByteArray relUtf8 = relPath.toUtf8();
    uint8_t* bytes = nullptr;
    size_t len = 0;
    if (lda_addon_read_asset(m_registry, idUtf8.constData(), relUtf8.constData(), &bytes, &len) != LDA_OK)
        return {};

    return takeBytes(bytes, len);
}

QByteArray AddonBridge::entryScript(const QString& id)
{
    if (!m_registry)
        return {};

    const QByteArray idUtf8 = id.toUtf8();
    uint8_t* bytes = nullptr;
    size_t len = 0;
    if (lda_addon_entry_script(m_registry, idUtf8.constData(), &bytes, &len) != LDA_OK)
        return {};

    return takeBytes(bytes, len);
}

// ─── Helpers ───────────────────────────────────────────

QString AddonBridge::errorMessage(const QJsonObject& json)
{
    const QJsonObject error = json.value(QLatin1String("error")).toObject();
    if (error.isEmpty())
        return {};
    return error.value(QLatin1String("message")).toString(error.value(QLatin1String("code")).toString());
}
