#ifndef ADDONPERMISSIONS_H
#define ADDONPERMISSIONS_H

#include <QString>
#include <QStringList>
#include <QSet>

class AddonPermissions
{
public:
    enum Permission
    {
        None            = 0,
        Filesystem      = 1 << 0,
        Network         = 1 << 1,
        Execute         = 1 << 2,
        Clipboard       = 1 << 3,
        Notifications   = 1 << 4,
        ProjectAccess   = 1 << 5,
        UiAccess        = 1 << 6,
        AllPermissions  = (1 << 7) - 1
    };
    Q_DECLARE_FLAGS(Permissions, Permission)

    static Permission fromString(const QString &name);
    static QString toString(Permission perm);
    static QString description(Permission perm);
    static Permissions fromStringList(const QStringList &list);
    static QStringList toStringList(Permissions perms);

    static bool isDangerous(Permission perm);
    static QStringList dangerousPermissions(Permissions perms);
};

Q_DECLARE_OPERATORS_FOR_FLAGS(AddonPermissions::Permissions)

#endif
