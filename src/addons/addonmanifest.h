#ifndef ADDONMANIFEST_H
#define ADDONMANIFEST_H

#include <QString>
#include <QStringList>
#include <QJsonObject>
#include <QJsonArray>
#include <yaml-cpp/yaml.h>

struct AddonManifest
{
    QString id;
    QString name;
    QString version;
    QString runtime;
    QString entry;
    int api = 1;
    QString author;
    QString description;
    QString license;
    QString website;
    QString icon;
    QString minVersion;
    QString maxVersion;
    QStringList categories;
    QStringList tags;
    QStringList dependencies;
    QStringList permissions;

    bool isValid = false;
    QString errorMessage;

    static AddonManifest fromFile(const QString &path);
    static AddonManifest fromYaml(const QString &content);
    static AddonManifest fromJson(const QString &content);

    bool validate();
    QString toYaml() const;

private:
    void parseYamlNode(const YAML::Node &node);
    void parseJsonObject(const QJsonObject &obj);
};

#endif
