#ifndef LUAADDONRUNTIME_H
#define LUAADDONRUNTIME_H

#include "addonruntime.h"

struct lua_State;

class LuaAddonRuntime : public AddonRuntime
{
    Q_OBJECT

public:
    LuaAddonRuntime(const AddonManifest &manifest, const QString &pluginDir,
                    AddonAPI *api, QObject *parent = nullptr);
    ~LuaAddonRuntime() override;

    bool load() override;
    bool enable() override;
    bool disable() override;
    bool unload() override;
    bool reload() override;

    bool dispatchEvent(AddonEvent event, const QJsonObject &data = {}) override;
    QJsonValue callFunction(const QString &name, const QJsonArray &args = {}) override;
    QJsonValue executeCommand(const QString &name, const QJsonArray &args = {}) override;

private:
    bool initLuaState();
    bool callLifecycleHook(const QString &hookName);
    void registerAPI();

    lua_State *m_lua = nullptr;
    AddonAPI *m_api = nullptr;

    // Command registry: Lua refs stored in the registry table
    QHash<QString, int> m_commandRefs;

public:
    void storeCommandRef(const QString &name, int ref);
    void releaseCommandRefs();
};

#endif
