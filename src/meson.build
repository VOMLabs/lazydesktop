qt6 = import('qt6')

yaml_cpp_dep = dependency('yaml-cpp', required: true)
llama_dep = dependency('llama', fallback: ['llama.cpp', 'llama_dep'], required: true)

processed = qt6.preprocess(
  moc_headers: ['mainwindow.h', 'diffviewer.h', 'llamaai.h'],
)

sources = files(
  'main.cpp',
  'mainwindow.cpp',
  'diffviewer.cpp',
  'llamaai.cpp',
)

executable('lazydesktop',
  sources,
  processed,
  dependencies: [qt6_dep, yaml_cpp_dep, llama_dep],
  install: true,
)
