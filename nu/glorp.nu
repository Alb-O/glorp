use ./completions.nu *

def glorp_socket [
  --socket: string = ""
  --session: any = null
] {
  if not ($socket | is-empty) {
    $socket
  } else {
    try {
      $session.socket
    } catch {
      ""
    }
  }
}

def glorp_cli_json [
  args: list<string>
  --socket: string = ""
  --session: any = null
] {
  let resolved_socket = (glorp_socket --socket=$socket --session=$session)
  let base = if ($resolved_socket | is-empty) { [] } else { ["--socket", $resolved_socket] }
  ^glorp_cli ...$base ...$args | from json
}

export def "glorp schema" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["schema"] --socket=$socket --session=$session
}

export def "glorp session attach" [--socket: string = ""] {
  glorp_cli_json ["session", "attach"] --socket=$socket
}

export def "glorp get config" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "config"] --socket=$socket --session=$session
}

export def "glorp get state" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "state"] --socket=$socket --session=$session
}

export def "glorp get document-text" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "document-text"] --socket=$socket --session=$session
}

export def "glorp get selection" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "selection"] --socket=$socket --session=$session
}

export def "glorp get inspect-details" [
  target?: string
  --socket: string = ""
  --session: any = null
] {
  let args = if ($target | is-empty) {
    ["get", "inspect-details"]
  } else {
    ["get", "inspect-details", "--target", $target]
  }
  glorp_cli_json $args --socket=$socket --session=$session
}

export def "glorp get perf" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "perf"] --socket=$socket --session=$session
}

export def "glorp get ui" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "ui"] --socket=$socket --session=$session
}

export def "glorp get capabilities" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["get", "capabilities"] --socket=$socket --session=$session
}

export def "glorp config set" [
  path: string
  value: any
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["config", "set", $path, ($value | to json -r)] --socket=$socket --session=$session
}

export def "glorp config reset" [
  path: string
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["config", "reset", $path] --socket=$socket --session=$session
}

export def "glorp config patch" [
  patch: any
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["config", "patch", ($patch | to json -r)] --socket=$socket --session=$session
}

export def "glorp config validate" [
  path: string
  value: any
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["config", "validate", $path, ($value | to json -r)] --socket=$socket --session=$session
}

export def "glorp config reload" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["config", "reload"] --socket=$socket --session=$session
}

export def "glorp config persist" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["config", "persist"] --socket=$socket --session=$session
}

export def "glorp doc replace" [
  text: string
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["doc", "replace", $text] --socket=$socket --session=$session
}

export def "glorp editor motion" [
  motion: string@"nu-complete glorp motion"
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["editor", "motion", $motion] --socket=$socket --session=$session
}

export def "glorp editor mode" [
  mode: string@"nu-complete glorp mode"
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["editor", "mode", $mode] --socket=$socket --session=$session
}

export def "glorp editor edit insert" [
  text: string
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["editor", "edit", "insert", $text] --socket=$socket --session=$session
}

export def "glorp editor edit backspace" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["editor", "edit", "backspace"] --socket=$socket --session=$session
}

export def "glorp editor edit delete-forward" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["editor", "edit", "delete-forward"] --socket=$socket --session=$session
}

export def "glorp editor edit delete-selection" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["editor", "edit", "delete-selection"] --socket=$socket --session=$session
}

export def "glorp editor history" [
  action: string@"nu-complete glorp history"
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["editor", "history", $action] --socket=$socket --session=$session
}

export def "glorp ui sidebar select" [
  tab: string@"nu-complete glorp tab"
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["ui", "sidebar", "select", $tab] --socket=$socket --session=$session
}

export def "glorp ui viewport scroll-to" [
  x: number
  y: number
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["ui", "viewport", "scroll-to", ($x | into string), ($y | into string)] --socket=$socket --session=$session
}

export def "glorp ui pane-ratio-set" [
  ratio: number
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["ui", "pane-ratio-set", ($ratio | into string)] --socket=$socket --session=$session
}

export def "glorp scene ensure" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["scene", "ensure"] --socket=$socket --session=$session
}

export def "glorp events subscribe" [--socket: string = "" --session: any = null] {
  glorp_cli_json ["events", "subscribe"] --socket=$socket --session=$session
}

export def "glorp events next" [
  token: int
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["events", "next", ($token | into string)] --socket=$socket --session=$session
}

export def "glorp events unsubscribe" [
  token: int
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["events", "unsubscribe", ($token | into string)] --socket=$socket --session=$session
}

export def "glorp txn" [
  commands: list<any>
  --socket: string = ""
  --session: any = null
] {
  glorp_cli_json ["txn", ({commands: $commands} | to json -r)] --socket=$socket --session=$session
}

export def "glorp cmd config set" [path: string value: any] {
  {Config: {Set: {path: $path value: $value}}}
}

export def "glorp cmd config reset" [path: string] {
  {Config: {Reset: {path: $path}}}
}

export def "glorp cmd doc replace" [text: string] {
  {Document: {Replace: {text: $text}}}
}

export def "glorp cmd editor motion" [motion: string@"nu-complete glorp motion"] {
  {Editor: {Motion: $motion}}
}

export def "glorp cmd editor mode" [mode: string@"nu-complete glorp mode"] {
  {Editor: {Mode: $mode}}
}

export def "glorp cmd editor edit insert" [text: string] {
  {Editor: {Edit: {Insert: {text: $text}}}}
}

export def "glorp cmd editor history" [action: string@"nu-complete glorp history"] {
  {Editor: {History: $action}}
}

export def "glorp cmd ui sidebar select" [tab: string@"nu-complete glorp tab"] {
  {Ui: {SidebarSelect: {tab: $tab}}}
}

export def "glorp cmd ui viewport scroll-to" [x: number y: number] {
  {Ui: {ViewportScrollTo: {x: $x y: $y}}}
}

export def "glorp cmd scene ensure" [] {
  {Scene: "Ensure"}
}

export def "glorp open-inspect" [--socket: string = "" --session: any = null] {
  glorp ui sidebar select inspect --socket=$socket --session=$session
}

export def "glorp open-perf" [--socket: string = "" --session: any = null] {
  glorp ui sidebar select perf --socket=$socket --session=$session
}

export def "glorp scroll-to-top" [--socket: string = "" --session: any = null] {
  glorp ui viewport scroll-to 0 0 --socket=$socket --session=$session
}

export def "glorp undo" [--socket: string = "" --session: any = null] {
  glorp editor history undo --socket=$socket --session=$session
}

export def "glorp redo" [--socket: string = "" --session: any = null] {
  glorp editor history redo --socket=$socket --session=$session
}
