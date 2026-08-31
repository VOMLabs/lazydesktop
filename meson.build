project('lazydesktop', 'cpp',
  version : '0.2.0',
  default_options : [
    'cpp_std=c++23',
    'buildtype=debugoptimized',
    'warning_level=3',
  ])

# ── Dependencies ────────────────────────────────────────────────
qt6 = import('qt6')
qt6_dep = dependency('qt6', modules : ['Core', 'Gui', 'Widgets', 'Network'])
yaml_dep = dependency('yaml-cpp', version : '>=0.7.0', fallback : 'yaml-cpp')

# ── Include directories ─────────────────────────────────────────
# Rust FFI C ABI headers
rust_incs = include_directories(
  'crates/ai_core',
  'crates/vcs_core',
  'crates/addons/include',
  'crates/watcher/include',
  'crates/config/include',
  'crates/git_cmd/include',
)

# ── Rust crate static libraries ────────────────────────────────
# The profile subdirectory is determined by the Meson build type.
if get_option('buildtype') == 'release'
  rust_profile = 'release'
else
  rust_profile = 'debug'
endif

rust_target_dir = meson.project_source_root() / 'target' / rust_profile

# Find pre-built Rust static libraries
cc = meson.get_compiler('cpp')
ai_core_lib = cc.find_library('ai_core',
  dirs : [rust_target_dir], static : true)
vcs_core_lib = cc.find_library('vcs_core',
  dirs : [rust_target_dir], static : true)
addons_lib = cc.find_library('addons',
  dirs : [rust_target_dir], static : true)
watcher_lib = cc.find_library('watcher',
  dirs : [rust_target_dir], static : true)
config_lib = cc.find_library('config',
  dirs : [rust_target_dir], static : true)
git_cmd_lib = cc.find_library('git_cmd',
  dirs : [rust_target_dir], static : true)

# ── MOC processing (Qt Meta-Object Compiler) ────────────────────
# Headers containing Q_OBJECT macros need MOC to generate vtable,
# staticMetaObject, and signal/slot glue code.
moc_headers = files(
  'src/mainwindow.h',
  'src/diffviewer.h',
  'src/addon_bridge.h',
  'src/model_manager_bridge.h',
  'src/vcs_bridge.h',
)

moc_sources = qt6.compile_moc(
  headers : moc_headers,
  dependencies : qt6_dep,
)

# ── C++ sources ─────────────────────────────────────────────────
src = files(
  'src/main.cpp',
  'src/mainwindow.cpp',
  'src/diffviewer.cpp',
  'src/addon_bridge.cpp',
  'src/background_download.cpp',
  'src/model_manager_bridge.cpp',
  'src/vcs_bridge.cpp',
)

# ── Link flags per platform ─────────────────────────────────────
extra_link_args = []
extra_cxx_args = []

if host_machine.system() == 'linux'
  extra_link_args += [
    '-Wl,--allow-multiple-definition',
  ]
elif host_machine.system() == 'darwin'
  extra_link_args += [
    '-Wl,-allow_multiple_definition',
  ]
elif host_machine.system() == 'windows'
  extra_cxx_args += ['/EHsc']
endif

# ── Build executable ────────────────────────────────────────────
executable('lazydesktop',
  src,
  moc_sources,
  include_directories : rust_incs,
  dependencies : [
    qt6_dep,
    yaml_dep,
    ai_core_lib,
    vcs_core_lib,
    addons_lib,
    watcher_lib,
    config_lib,
    git_cmd_lib,
  ],
  link_args : extra_link_args,
  cpp_args : extra_cxx_args,
  install : true,
)
