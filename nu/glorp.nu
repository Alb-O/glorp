plugin use glorp
use ./completions.nu *

export def "glorp open-inspect" [] {
  glorp ui sidebar select inspect
}

export def "glorp open-perf" [] {
  glorp ui sidebar select perf
}

export def "glorp scroll-to-top" [] {
  glorp ui viewport scroll-to 0 0
}

export def "glorp undo" [] {
  glorp editor history undo
}

export def "glorp redo" [] {
  glorp editor history redo
}
