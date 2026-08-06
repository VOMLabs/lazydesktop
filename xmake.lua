add_rules("mode.debug", "mode.release")

set_languages("cxx23")
set_project("lazydesktop")
set_version("0.1.0")
set_allowedplats("linux", "windows", "macosx")

add_requires("yaml-cpp", {system = true})

target("lazydesktop")
    set_kind("binary")
    add_rules("qt.widgetapp")

    add_files("src/*.cpp")
    add_files("src/*.h")
    add_includedirs("crates/ai_core")

    add_frameworks("QtNetwork")
    add_packages("yaml-cpp")
    add_links("ai_core")

    if is_mode("release") then
        add_linkdirs(path.join(os.projectdir(), "target/release"))
    else
        add_linkdirs(path.join(os.projectdir(), "target/debug"))
    end

    before_build(function()
        local profile = is_mode("release") and "--release" or ""
        os.exec("cargo build --lib --manifest-path " .. os.projectdir() .. "/crates/ai_core/Cargo.toml " .. profile)
    end)

    if is_plat("linux") then
        add_syslinks("pthread", "dl", "rt", "gomp")
    elseif is_plat("macosx") then
        add_syslinks("pthread")
        add_frameworks("OpenMP")
    elseif is_plat("windows") then
        add_cxflags("/EHsc")
    end
