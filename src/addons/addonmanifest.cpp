#include "addonmanifest.h"
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QFileInfo>
#include <yaml-cpp/yaml.h>

AddonManifest AddonManifest::fromFile(const QString &path)
{
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly)) {
        AddonManifest m;
        m.isValid = false;
        m.errorMessage = "Cannot open manifest file: " + path;
        return m;
    }

    QString content = QString::fromUtf8(file.readAll());
    file.close();

    if (path.endsWith(".json", Qt::CaseInsensitive))
        return fromJson(content);
    return fromYaml(content);
}

AddonManifest AddonManifest::fromYaml(const QString &content)
{
    AddonManifest m;
    try {
        YAML::Node root = YAML::Load(content.toStdString());
        m.parseYamlNode(root);
    } catch (const YAML::Exception &e) {
        m.isValid = false;
        m.errorMessage = QString::fromStdString(e.what());
        return m;
    }
    m.validate();
    return m;
}

AddonManifest AddonManifest::fromJson(const QString &content)
{
    AddonManifest m;
    QJsonParseError err;
    QJsonDocument doc = QJsonDocument::fromJson(content.toUtf8(), &err);
    if (err.error != QJsonParseError::NoError) {
        m.isValid = false;
        m.errorMessage = "JSON parse error: " + err.errorString();
        return m;
    }
    if (!doc.isObject()) {
        m.isValid = false;
        m.errorMessage = "JSON root is not an object";
        return m;
    }
    m.parseJsonObject(doc.object());
    m.validate();
    return m;
}

bool AddonManifest::validate()
{
    if (id.isEmpty()) {
        isValid = false;
        errorMessage = "Plugin ID is required";
        return false;
    }
    if (name.isEmpty()) {
        isValid = false;
        errorMessage = "Plugin name is required";
        return false;
    }
    if (version.isEmpty()) {
        isValid = false;
        errorMessage = "Plugin version is required";
        return false;
    }
    if (runtime.isEmpty()) {
        isValid = false;
        errorMessage = "Runtime is required (lua or python)";
        return false;
    }
    if (runtime != "lua" && runtime != "python") {
        isValid = false;
        errorMessage = "Runtime must be 'lua' or 'python', got: " + runtime;
        return false;
    }
    if (entry.isEmpty()) {
        isValid = false;
        errorMessage = "Entry point is required";
        return false;
    }
    if (api < 1) {
        isValid = false;
        errorMessage = "API version must be >= 1";
        return false;
    }
    isValid = true;
    errorMessage.clear();
    return true;
}

void AddonManifest::parseYamlNode(const YAML::Node &root)
{
    if (!root.IsMap()) return;

    auto readStr = [&](const QString &key) -> QString {
        auto node = root[key.toStdString()];
        if (node && node.IsScalar())
            return QString::fromStdString(node.Scalar());
        return {};
    };

    auto readStrList = [&](const QString &key) -> QStringList {
        auto node = root[key.toStdString()];
        QStringList list;
        if (node && node.IsSequence()) {
            for (const auto &item : node)
                list.append(QString::fromStdString(item.Scalar()));
        }
        return list;
    };

    id = readStr("id");
    name = readStr("name");
    version = readStr("version");
    runtime = readStr("runtime");
    entry = readStr("entry");
    author = readStr("author");
    description = readStr("description");
    license = readStr("license");
    website = readStr("website");
    icon = readStr("icon");
    minVersion = readStr("min_version");
    maxVersion = readStr("max_version");

    if (root["api"] && root["api"].IsScalar())
        api = root["api"].as<int>();

    categories = readStrList("categories");
    tags = readStrList("tags");
    dependencies = readStrList("dependencies");
    permissions = readStrList("permissions");
}

void AddonManifest::parseJsonObject(const QJsonObject &obj)
{
    id = obj["id"].toString();
    name = obj["name"].toString();
    version = obj["version"].toString();
    runtime = obj["runtime"].toString();
    entry = obj["entry"].toString();
    api = obj["api"].toInt(1);
    author = obj["author"].toString();
    description = obj["description"].toString();
    license = obj["license"].toString();
    website = obj["website"].toString();
    icon = obj["icon"].toString();
    minVersion = obj["min_version"].toString();
    maxVersion = obj["max_version"].toString();

    for (const auto &v : obj["categories"].toArray())
        categories.append(v.toString());
    for (const auto &v : obj["tags"].toArray())
        tags.append(v.toString());
    for (const auto &v : obj["dependencies"].toArray())
        dependencies.append(v.toString());
    for (const auto &v : obj["permissions"].toArray())
        permissions.append(v.toString());
}

QString AddonManifest::toYaml() const
{
    YAML::Node root;

    auto setStr = [&](const QString &key, const QString &val) {
        if (!val.isEmpty())
            root[key.toStdString()] = val.toStdString();
    };

    setStr("id", id);
    setStr("name", name);
    setStr("version", version);
    setStr("runtime", runtime);
    setStr("entry", entry);
    root["api"] = api;
    setStr("author", author);
    setStr("description", description);
    setStr("license", license);
    setStr("website", website);
    setStr("icon", icon);
    setStr("min_version", minVersion);
    setStr("max_version", maxVersion);

    auto setList = [&](const QString &key, const QStringList &list) {
        if (list.isEmpty()) return;
        for (const auto &item : list)
            root[key.toStdString()].push_back(item.toStdString());
    };

    setList("categories", categories);
    setList("tags", tags);
    setList("dependencies", dependencies);
    setList("permissions", permissions);

    YAML::Emitter out;
    out << root;
    return QString::fromStdString(out.c_str());
}
