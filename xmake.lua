add_rules("mode.debug", "mode.release")

set_languages("cxx23")
set_project("lazydesktop")
set_version("0.2.0")
set_allowedplats("linux", "windows", "macosx")

-- Let xmake use the system yaml-cpp when available (apt/brew/pacman);
-- otherwise it fetches and builds yaml-cpp from the xmake package repo.
-- This replaces the previous `{system = true}` which could not discover
-- yaml-cpp installed via vcpkg on Windows.
add_requires("yaml-cpp")

target("lazydesktop")
    set_kind("binary")
    add_rules("qt.widgetapp")

    add_files("src/*.cpp")
    add_files("src/*.h")
    add_includedirs("crates/ai_core")
    add_includedirs("crates/vcs_core")
    add_includedirs("crates/addons/include")
    add_includedirs("crates/watcher/include")
    add_includedirs("crates/config/include")
    add_includedirs("crates/git_cmd/include")

    add_frameworks("QtNetwork")
    add_packages("yaml-cpp")
    add_links("ai_core")
    add_links("vcs_core")
    add_links("addons")
    add_links("watcher")
    add_links("config")
    add_links("git_cmd")

    if is_mode("release") then
        add_linkdirs(path.join(os.projectdir(), "target/release"))
    else
        add_linkdirs(path.join(os.projectdir(), "target/debug"))
    end

    before_build(function()
        local profile = is_mode("release") and "--release" or ""
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/ai_core/Cargo.toml " .. profile)
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/vcs_core/Cargo.toml " .. profile)
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/addons/Cargo.toml " .. profile)
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/watcher/Cargo.toml " .. profile)
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/config/Cargo.toml " .. profile)
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/git_cmd/Cargo.toml " .. profile)
    end)

    if is_plat("linux") then
        add_syslinks("pthread", "dl", "rt", "gomp")
        -- Both Rust staticlibs (ai_core, vcs_core) embed a copy of Rust's std.
        -- Linking them into one binary surfaces duplicate std symbols; they are
        -- identical (same toolchain), so keep the first definition.
        add_ldflags("-Wl,--allow-multiple-definition")
    elseif is_plat("macosx") then
        add_syslinks("pthread")
        add_ldflags("-Wl,-allow_multiple_definition")
    elseif is_plat("windows") then
        add_cxflags("/EHsc")
    end
