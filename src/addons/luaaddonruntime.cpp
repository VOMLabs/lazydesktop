#include "luaaddonruntime.h"
#include "addonmanifest.h"
#include "addonapi.h"

#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonArray>
#include <QNetworkReply>
#include <QEventLoop>

extern "C" {
#include <lua.h>
#include <lauxlib.h>
#include <lualib.h>
}

LuaAddonRuntime::LuaAddonRuntime(const AddonManifest &manifest, const QString &pluginDir,
                                 AddonAPI *api, QObject *parent)
    : AddonRuntime(manifest, pluginDir, parent)
    , m_api(api)
{
}

LuaAddonRuntime::~LuaAddonRuntime()
{
    unload();
}

bool LuaAddonRuntime::load()
{
    if (m_lua) return true;

    m_status = AddonStatus::Loading;

    if (!initLuaState()) {
        m_status = AddonStatus::Error;
        m_lastError = "Failed to initialize Lua state";
        return false;
    }

    QString entryPath = m_pluginDir + "/" + m_manifest.entry;
    QFile file(entryPath);
    if (!file.open(QIODevice::ReadOnly)) {
        m_lastError = "Cannot open entry file: " + entryPath;
        m_status = AddonStatus::Error;
        return false;
    }

    QByteArray script = file.readAll();
    file.close();

    if (luaL_loadstring(m_lua, script.constData()) != LUA_OK) {
        m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
        lua_pop(m_lua, 1);
        m_status = AddonStatus::Error;
        return false;
    }

    if (lua_pcall(m_lua, 0, 0, 0) != LUA_OK) {
        m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
        lua_pop(m_lua, 1);
        m_status = AddonStatus::Error;
        return false;
    }

    if (!callLifecycleHook("onLoad")) {
        lua_close(m_lua);
        m_lua = nullptr;
        m_status = AddonStatus::Error;
        return false;
    }

    m_status = AddonStatus::Loaded;
    return true;
}

bool LuaAddonRuntime::enable()
{
    if (m_status == AddonStatus::Enabled) return true;
    if (m_status != AddonStatus::Loaded && !load())
        return false;

    if (!callLifecycleHook("onEnable")) {
        return false;
    }

    m_status = AddonStatus::Enabled;
    emit statusChanged(m_status);
    return true;
}

bool LuaAddonRuntime::disable()
{
    if (m_status != AddonStatus::Enabled) return true;

    callLifecycleHook("onDisable");
    m_status = AddonStatus::Loaded;
    emit statusChanged(m_status);
    return true;
}

bool LuaAddonRuntime::unload()
{
    if (!m_lua) return true;

    if (m_status == AddonStatus::Enabled)
        callLifecycleHook("onDisable");
    callLifecycleHook("onUnload");

    releaseCommandRefs();
    lua_close(m_lua);
    m_lua = nullptr;
    m_status = AddonStatus::Installed;
    emit statusChanged(m_status);
    return true;
}

bool LuaAddonRuntime::reload()
{
    bool wasEnabled = (m_status == AddonStatus::Enabled);
    unload();
    if (!load())
        return false;
    if (wasEnabled && !enable())
        return false;

    emit statusChanged(m_status);
    return true;
}

bool LuaAddonRuntime::dispatchEvent(AddonEvent event, const QJsonObject &data)
{
    static const QHash<AddonEvent, QString> eventNames = {
        {AddonEvent::AppStarted,       "onAppStarted"},
        {AddonEvent::AppShuttingDown,  "onAppShuttingDown"},
        {AddonEvent::ProjectOpened,    "onProjectOpened"},
        {AddonEvent::ProjectClosed,    "onProjectClosed"},
        {AddonEvent::ThemeChanged,     "onThemeChanged"},
        {AddonEvent::FileOpened,       "onFileOpened"},
        {AddonEvent::FileSaved,        "onFileSaved"},
        {AddonEvent::WindowCreated,    "onWindowCreated"},
        {AddonEvent::WindowClosed,     "onWindowClosed"},
        {AddonEvent::SettingsChanged,  "onSettingsChanged"},
    };

    QString hookName = eventNames.value(event);
    if (hookName.isEmpty())
        return true;

    lua_getglobal(m_lua, hookName.toUtf8().constData());
    if (!lua_isfunction(m_lua, -1)) {
        lua_pop(m_lua, 1);
        return true;
    }

    if (!data.isEmpty()) {
        QByteArray json = QJsonDocument(data).toJson(QJsonDocument::Compact);
        lua_pushlstring(m_lua, json.constData(), json.size());
        if (lua_pcall(m_lua, 1, 1, 0) != LUA_OK) {
            m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
            lua_pop(m_lua, 1);
            emit errorOccurred(m_lastError);
            return false;
        }
    } else {
        if (lua_pcall(m_lua, 0, 1, 0) != LUA_OK) {
            m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
            lua_pop(m_lua, 1);
            emit errorOccurred(m_lastError);
            return false;
        }
    }

    lua_pop(m_lua, 1);
    return true;
}

QJsonValue LuaAddonRuntime::callFunction(const QString &name, const QJsonArray &args)
{
    lua_getglobal(m_lua, name.toUtf8().constData());
    if (!lua_isfunction(m_lua, -1)) {
        lua_pop(m_lua, 1);
        return {};
    }

    for (const auto &arg : args) {
        switch (arg.type()) {
        case QJsonValue::String:
            lua_pushstring(m_lua, arg.toString().toUtf8().constData());
            break;
        case QJsonValue::Double:
            lua_pushnumber(m_lua, arg.toDouble());
            break;
        case QJsonValue::Bool:
            lua_pushboolean(m_lua, arg.toBool());
            break;
        case QJsonValue::Array:
        case QJsonValue::Object: {
            QJsonDocument doc;
            if (arg.isObject())
                doc = QJsonDocument(arg.toObject());
            else
                doc = QJsonDocument(arg.toArray());
            QByteArray json = doc.toJson(QJsonDocument::Compact);
            lua_pushlstring(m_lua, json.constData(), json.size());
            break;
        }
        default:
            lua_pushnil(m_lua);
            break;
        }
    }

    if (lua_pcall(m_lua, args.size(), 1, 0) != LUA_OK) {
        m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
        lua_pop(m_lua, 1);
        emit errorOccurred(m_lastError);
        return {};
    }

    QJsonValue result;
    if (lua_isstring(m_lua, -1))
        result = QString::fromUtf8(lua_tostring(m_lua, -1));
    else if (lua_isnumber(m_lua, -1))
        result = lua_tonumber(m_lua, -1);
    else if (lua_isboolean(m_lua, -1))
        result = (bool)lua_toboolean(m_lua, -1);
    else if (lua_istable(m_lua, -1))
        result = QJsonValue(QJsonObject{{"value", QString::fromUtf8(lua_tostring(m_lua, -1))}});

    lua_pop(m_lua, 1);
    return result;
}

bool LuaAddonRuntime::initLuaState()
{
    m_lua = luaL_newstate();
    if (!m_lua)
        return false;

    luaL_openlibs(m_lua);

    QString packagePath = QString("package.path = '%1/src/?.lua;' .. package.path")
                              .arg(m_pluginDir);
    if (luaL_loadstring(m_lua, packagePath.toUtf8().constData()) == LUA_OK)
        lua_pcall(m_lua, 0, 0, 0);

    registerAPI();
    return true;
}

bool LuaAddonRuntime::callLifecycleHook(const QString &hookName)
{
    if (!m_lua) return false;

    lua_getglobal(m_lua, hookName.toUtf8().constData());
    if (!lua_isfunction(m_lua, -1)) {
        lua_pop(m_lua, 1);
        return true;
    }

    if (lua_pcall(m_lua, 0, 0, 0) != LUA_OK) {
        m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
        lua_pop(m_lua, 1);
        emit errorOccurred(m_lastError);
        return false;
    }

    return true;
}

static int lua_addon_log(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *msg = lua_tostring(L, 1);
    const char *level = lua_isstring(L, 2) ? lua_tostring(L, 2) : "info";
    if (msg) api->log(QString::fromUtf8(msg), QString::fromUtf8(level));
    return 0;
}

static int lua_addon_read_config(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *key = lua_tostring(L, 1);
    const char *def = lua_isstring(L, 2) ? lua_tostring(L, 2) : "";
    QString val = api->readConfig(QString::fromUtf8(key), QString::fromUtf8(def));
    lua_pushstring(L, val.toUtf8().constData());
    return 1;
}

static int lua_addon_write_config(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *key = lua_tostring(L, 1);
    const char *val = lua_tostring(L, 2);
    if (key && val) api->writeConfig(QString::fromUtf8(key), QString::fromUtf8(val));
    return 0;
}

static int lua_addon_read_asset(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *path = lua_tostring(L, 1);
    if (!path) { lua_pushnil(L); return 1; }
    QByteArray data = api->readAsset(QString::fromUtf8(path));
    lua_pushlstring(L, data.constData(), data.size());
    return 1;
}

static int lua_addon_data_path(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *sub = lua_isstring(L, 1) ? lua_tostring(L, 1) : "";
    QString path = api->dataPath(QString::fromUtf8(sub));
    lua_pushstring(L, path.toUtf8().constData());
    return 1;
}

static int lua_addon_asset_path(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *rel = lua_tostring(L, 1);
    if (!rel) { lua_pushnil(L); return 1; }
    QString path = api->assetPath(QString::fromUtf8(rel));
    lua_pushstring(L, path.toUtf8().constData());
    return 1;
}

static int lua_addon_notify(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *title = lua_tostring(L, 1);
    const char *msg = lua_tostring(L, 2);
    if (title && msg)
        emit api->notify(QString::fromUtf8(title), QString::fromUtf8(msg));
    return 0;
}

static int lua_addon_http_get(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *url = lua_tostring(L, 1);
    if (!url) { lua_pushnil(L); return 1; }

    auto *reply = api->network()->get(QNetworkRequest(QUrl(QString::fromUtf8(url))));
    QEventLoop loop;
    QObject::connect(reply, &QNetworkReply::finished, &loop, &QEventLoop::quit);
    loop.exec();

    if (reply->error() == QNetworkReply::NoError) {
        QByteArray data = reply->readAll();
        lua_pushlstring(L, data.constData(), data.size());
    } else {
        lua_pushnil(L);
    }
    reply->deleteLater();
    return 1;
}

// --- New Lua API functions ---

static int lua_addon_register_command(lua_State *L)
{
    auto *runtime = (LuaAddonRuntime *)lua_touserdata(L, lua_upvalueindex(1));
    const char *name = lua_tostring(L, 1);
    if (!name || !lua_isfunction(L, 2)) {
        lua_pushboolean(L, 0);
        return 1;
    }

    QString cmdName = QString::fromUtf8(name);

    // Store the Lua function reference in the registry
    lua_pushvalue(L, 2); // duplicate the function
    int ref = luaL_ref(L, LUA_REGISTRYINDEX);
    runtime->storeCommandRef(cmdName, ref);

    // Register with the addon manager
    auto *api = runtime->api();
    bool ok = api->registerCommand(cmdName, [runtime, cmdName](const QJsonArray &args) -> QJsonValue {
        return runtime->executeCommand(cmdName, args);
    });

    lua_pushboolean(L, ok);
    return 1;
}

static int lua_addon_create_file(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *path = lua_tostring(L, 1);
    size_t contentLen;
    const char *content = lua_tolstring(L, 2, &contentLen);
    if (!path || !content) { lua_pushboolean(L, 0); return 1; }
    bool ok = api->createFile(QString::fromUtf8(path), QByteArray(content, contentLen));
    lua_pushboolean(L, ok);
    return 1;
}

static int lua_addon_read_file(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *path = lua_tostring(L, 1);
    if (!path) { lua_pushnil(L); return 1; }
    QByteArray data = api->readFileData(QString::fromUtf8(path));
    if (data.isEmpty()) { lua_pushnil(L); return 1; }
    lua_pushlstring(L, data.constData(), data.size());
    return 1;
}

static int lua_addon_write_file(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *path = lua_tostring(L, 1);
    size_t contentLen;
    const char *content = lua_tolstring(L, 2, &contentLen);
    if (!path || !content) { lua_pushboolean(L, 0); return 1; }
    bool ok = api->writeFile(QString::fromUtf8(path), QByteArray(content, contentLen));
    lua_pushboolean(L, ok);
    return 1;
}

static int lua_addon_delete_file(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *path = lua_tostring(L, 1);
    if (!path) { lua_pushboolean(L, 0); return 1; }
    bool ok = api->deleteFileData(QString::fromUtf8(path));
    lua_pushboolean(L, ok);
    return 1;
}

static int lua_addon_list_files(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *dir = lua_isstring(L, 1) ? lua_tostring(L, 1) : "";
    QStringList files = api->listFiles(QString::fromUtf8(dir));
    lua_createtable(L, files.size(), 0);
    for (int i = 0; i < files.size(); i++) {
        lua_pushstring(L, files[i].toUtf8().constData());
        lua_rawseti(L, -2, i + 1);
    }
    return 1;
}

static int lua_addon_create_menu(lua_State *L)
{
    auto *api = (AddonAPI *)lua_touserdata(L, lua_upvalueindex(1));
    const char *parentPath = lua_tostring(L, 1);
    const char *label = lua_tostring(L, 2);
    const char *commandId = lua_tostring(L, 3);
    if (!parentPath || !label || !commandId) { lua_pushboolean(L, 0); return 1; }
    bool ok = api->createMenu(QString::fromUtf8(parentPath), QString::fromUtf8(label), QString::fromUtf8(commandId));
    lua_pushboolean(L, ok);
    return 1;
}

QJsonValue LuaAddonRuntime::executeCommand(const QString &name, const QJsonArray &args)
{
    if (!m_lua) return {};

    int ref = m_commandRefs.value(name, LUA_REFNIL);
    if (ref == LUA_REFNIL) return {};

    lua_rawgeti(m_lua, LUA_REGISTRYINDEX, ref);
    if (!lua_isfunction(m_lua, -1)) {
        lua_pop(m_lua, 1);
        return {};
    }

    // Push args as a Lua table
    lua_createtable(m_lua, args.size(), 0);
    for (int i = 0; i < args.size(); i++) {
        switch (args[i].type()) {
        case QJsonValue::String:
            lua_pushstring(m_lua, args[i].toString().toUtf8().constData());
            break;
        case QJsonValue::Double:
            lua_pushnumber(m_lua, args[i].toDouble());
            break;
        case QJsonValue::Bool:
            lua_pushboolean(m_lua, args[i].toBool());
            break;
        default:
            lua_pushnil(m_lua);
            break;
        }
        lua_rawseti(m_lua, -2, i + 1);
    }

    if (lua_pcall(m_lua, 1, 1, 0) != LUA_OK) {
        m_lastError = QString::fromUtf8(lua_tostring(m_lua, -1));
        lua_pop(m_lua, 1);
        emit errorOccurred(m_lastError);
        return {};
    }

    QJsonValue result;
    if (lua_isstring(m_lua, -1))
        result = QString::fromUtf8(lua_tostring(m_lua, -1));
    else if (lua_isnumber(m_lua, -1))
        result = lua_tonumber(m_lua, -1);
    else if (lua_isboolean(m_lua, -1))
        result = (bool)lua_toboolean(m_lua, -1);
    lua_pop(m_lua, 1);
    return result;
}

void LuaAddonRuntime::storeCommandRef(const QString &name, int ref)
{
    releaseCommandRefs();
    m_commandRefs[name] = ref;
}

void LuaAddonRuntime::releaseCommandRefs()
{
    if (!m_lua) return;
    for (auto it = m_commandRefs.begin(); it != m_commandRefs.end(); ++it)
        luaL_unref(m_lua, LUA_REGISTRYINDEX, it.value());
    m_commandRefs.clear();
}

void LuaAddonRuntime::registerAPI()
{
    lua_newtable(m_lua);

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_log, 1);
    lua_setfield(m_lua, -2, "log");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_read_config, 1);
    lua_setfield(m_lua, -2, "readConfig");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_write_config, 1);
    lua_setfield(m_lua, -2, "writeConfig");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_read_asset, 1);
    lua_setfield(m_lua, -2, "readAsset");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_data_path, 1);
    lua_setfield(m_lua, -2, "dataPath");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_asset_path, 1);
    lua_setfield(m_lua, -2, "assetPath");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_notify, 1);
    lua_setfield(m_lua, -2, "notify");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_http_get, 1);
    lua_setfield(m_lua, -2, "httpGet");

    // --- New API functions ---
    lua_pushlightuserdata(m_lua, this);
    lua_pushcclosure(m_lua, lua_addon_register_command, 1);
    lua_setfield(m_lua, -2, "registerCommand");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_create_file, 1);
    lua_setfield(m_lua, -2, "createFile");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_read_file, 1);
    lua_setfield(m_lua, -2, "readFile");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_write_file, 1);
    lua_setfield(m_lua, -2, "writeFile");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_delete_file, 1);
    lua_setfield(m_lua, -2, "deleteFile");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_list_files, 1);
    lua_setfield(m_lua, -2, "listFiles");

    lua_pushlightuserdata(m_lua, m_api);
    lua_pushcclosure(m_lua, lua_addon_create_menu, 1);
    lua_setfield(m_lua, -2, "createMenu");

    lua_setglobal(m_lua, "addon");
}
