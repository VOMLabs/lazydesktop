#include "addonpermissions.h"

AddonPermissions::Permission AddonPermissions::fromString(const QString &name)
{
    static const QHash<QString, Permission> map = {
        {QStringLiteral("filesystem"),     Filesystem},
        {QStringLiteral("network"),        Network},
        {QStringLiteral("execute"),        Execute},
        {QStringLiteral("clipboard"),      Clipboard},
        {QStringLiteral("notifications"),  Notifications},
        {QStringLiteral("project_access"), ProjectAccess},
        {QStringLiteral("ui_access"),      UiAccess},
    };
    return map.value(name.toLower().trimmed(), None);
}

QString AddonPermissions::toString(Permission perm)
{
    switch (perm) {
    case Filesystem:     return "filesystem";
    case Network:        return "network";
    case Execute:        return "execute";
    case Clipboard:      return "clipboard";
    case Notifications:  return "notifications";
    case ProjectAccess:  return "project_access";
    case UiAccess:       return "ui_access";
    default:             return {};
    }
}

QString AddonPermissions::description(Permission perm)
{
    switch (perm) {
    case Filesystem:     return "Read and write files on your system";
    case Network:        return "Make network requests to remote servers";
    case Execute:        return "Execute external programs on your system";
    case Clipboard:      return "Read and write to the system clipboard";
    case Notifications:  return "Display system notifications";
    case ProjectAccess:  return "Read project files and repository data";
    case UiAccess:       return "Modify the application user interface";
    default:             return {};
    }
}

AddonPermissions::Permissions AddonPermissions::fromStringList(const QStringList &list)
{
    Permissions perms = None;
    for (const auto &s : list)
        perms |= fromString(s);
    return perms;
}

QStringList AddonPermissions::toStringList(Permissions perms)
{
    QStringList list;
    for (int i = 1; i < AllPermissions; i <<= 1) {
        auto p = static_cast<Permission>(i);
        if (perms & p) {
            QString s = toString(p);
            if (!s.isEmpty()) list.append(s);
        }
    }
    return list;
}

bool AddonPermissions::isDangerous(Permission perm)
{
    switch (perm) {
    case Execute:
    case Network:
    case Clipboard:
        return true;
    default:
        return false;
    }
}

QStringList AddonPermissions::dangerousPermissions(Permissions perms)
{
    QStringList list;
    for (int i = 1; i < AllPermissions; i <<= 1) {
        auto p = static_cast<Permission>(i);
        if ((perms & p) && isDangerous(p))
            list.append(toString(p));
    }
    return list;
}
