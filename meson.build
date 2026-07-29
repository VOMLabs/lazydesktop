project('lazydesktop', 'cpp',
  version: '0.1.0',
  meson_version: '>= 1.0.0',
  default_options: [
    'cpp_std=c++23',
    'buildtype=debugoptimized',
    'warning_level=3',
  ],
)

qt6 = import('qt6')
cc = meson.get_compiler('cpp')

# Dependencies
qt6_dep  = dependency('qt6', modules: ['Core', 'Gui', 'Widgets', 'Network'])
yaml_dep = dependency('yaml-cpp')

# llama.cpp / ggml
llama_dep = dependency('llama', required: false)
if not llama_dep.found()
  llama_dep = declare_dependency(
    dependencies: [
      cc.find_library('llama'),
      cc.find_library('ggml'),
    ]
  )
endif

sources = []
headers = []

# Populates sources and headers from src/
subdir('src')

# Process Qt MOC headers
qt_processed = qt6.preprocess(
  moc_headers: headers,
)

executable('lazydesktop',
  sources,
  qt_processed,
  dependencies: [
    qt6_dep,
    yaml_dep,
    llama_dep,
  ],
)
