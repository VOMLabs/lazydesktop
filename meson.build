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

qt6_dep = dependency('qt6', modules: ['Core', 'Gui', 'Widgets', 'Network'])

subdir('src')
