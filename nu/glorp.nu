plugin use glorp
use ./completions.nu *

export def "glorp cmd config set" [path: string value: any] {
  {path: "glorp config set" input: {path: $path value: $value}}
}

export def "glorp cmd config reset" [path: string] {
  {path: "glorp config reset" input: {path: $path}}
}

export def "glorp cmd config patch" [patch: any] {
  {path: "glorp config patch" input: {patch: $patch}}
}

export def "glorp cmd config reload" [] {
  {path: "glorp config reload" input: null}
}

export def "glorp cmd config persist" [] {
  {path: "glorp config persist" input: null}
}

export def "glorp cmd doc replace" [text: string] {
  {path: "glorp doc replace" input: {text: $text}}
}

export def "glorp cmd editor motion" [motion: string@"nu-complete glorp motion"] {
  {path: "glorp editor motion" input: {motion: $motion}}
}

export def "glorp cmd editor mode" [mode: string@"nu-complete glorp mode"] {
  {path: "glorp editor mode" input: {mode: $mode}}
}

export def "glorp cmd editor edit insert" [text: string] {
  {path: "glorp editor edit insert" input: {text: $text}}
}

export def "glorp cmd editor edit backspace" [] {
  {path: "glorp editor edit backspace" input: null}
}

export def "glorp cmd editor edit delete-forward" [] {
  {path: "glorp editor edit delete-forward" input: null}
}

export def "glorp cmd editor edit delete-selection" [] {
  {path: "glorp editor edit delete-selection" input: null}
}

export def "glorp cmd editor history" [action: string@"nu-complete glorp history"] {
  {path: "glorp editor history" input: {action: $action}}
}

export def "glorp cmd ui sidebar select" [tab: string@"nu-complete glorp tab"] {
  {path: "glorp ui sidebar select" input: {tab: $tab}}
}

export def "glorp cmd ui viewport scroll-to" [x: number y: number] {
  {path: "glorp ui viewport scroll-to" input: {x: $x y: $y}}
}

export def "glorp cmd ui pane-ratio-set" [ratio: number] {
  {path: "glorp ui pane-ratio-set" input: {ratio: $ratio}}
}

export def "glorp cmd scene ensure" [] {
  {path: "glorp scene ensure" input: null}
}

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
