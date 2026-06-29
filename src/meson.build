qt6 = import('qt6')

yaml_cpp_dep = dependency('yaml-cpp', required: true)

processed = qt6.preprocess(
  moc_headers: ['mainwindow.h', 'diffviewer.h'],
)

sources = files(
  'main.cpp',
  'mainwindow.cpp',
  'diffviewer.cpp',
)

executable('lazydesktop',
  sources,
  processed,
  dependencies: [qt6_dep, yaml_cpp_dep],
  install: true,
)
