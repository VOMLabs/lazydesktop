qt6 = import('qt6')

processed = qt6.preprocess(
  moc_headers: 'mainwindow.h',
)

sources = files(
  'main.cpp',
  'mainwindow.cpp',
)

executable('lazydesktop',
  sources,
  processed,
  dependencies: qt6_dep,
  install: true,
)
