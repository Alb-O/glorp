use ./completions.nu *

def glorp-cli-json [
  args: list<string>
  --socket: string = ""
] {
  let base = if ($socket | is-empty) { [] } else { ["--socket", $socket] }
  ^glorp-cli ...$base ...$args | from json
}

export def "glorp schema" [--socket: string = ""] {
  glorp-cli-json ["schema"] --socket=$socket
}

export def "glorp get config" [--socket: string = ""] {
  glorp-cli-json ["get", "config"] --socket=$socket
}

export def "glorp get state" [--socket: string = ""] {
  glorp-cli-json ["get", "state"] --socket=$socket
}

export def "glorp get document-text" [--socket: string = ""] {
  glorp-cli-json ["get", "document-text"] --socket=$socket
}

export def "glorp config set" [
  path: string
  value: string
  --socket: string = ""
] {
  glorp-cli-json ["config", "set", $path, $value] --socket=$socket
}

export def "glorp config reset" [
  path: string
  --socket: string = ""
] {
  glorp-cli-json ["config", "reset", $path] --socket=$socket
}

export def "glorp config patch" [
  patch: any
  --socket: string = ""
] {
  glorp-cli-json ["config", "patch", ($patch | to json -r)] --socket=$socket
}

export def "glorp config validate" [
  path: string
  value: any
  --socket: string = ""
] {
  glorp-cli-json ["config", "validate", $path, ($value | to json -r)] --socket=$socket
}

export def "glorp doc replace" [
  text: string
  --socket: string = ""
] {
  glorp-cli-json ["doc", "replace", $text] --socket=$socket
}

export def "glorp editor motion" [
  motion: string@"nu-complete glorp motion"
  --socket: string = ""
] {
  glorp-cli-json ["editor", "motion", $motion] --socket=$socket
}

export def "glorp editor mode" [
  mode: string@"nu-complete glorp mode"
  --socket: string = ""
] {
  glorp-cli-json ["editor", "mode", $mode] --socket=$socket
}

export def "glorp editor edit insert" [
  text: string
  --socket: string = ""
] {
  glorp-cli-json ["editor", "edit", "insert", $text] --socket=$socket
}

export def "glorp editor history" [
  action: string@"nu-complete glorp history"
  --socket: string = ""
] {
  glorp-cli-json ["editor", "history", $action] --socket=$socket
}

export def "glorp ui sidebar select" [
  tab: string@"nu-complete glorp tab"
  --socket: string = ""
] {
  glorp-cli-json ["ui", "sidebar", "select", $tab] --socket=$socket
}

export def "glorp ui viewport scroll-to" [
  x: number
  y: number
  --socket: string = ""
] {
  glorp-cli-json ["ui", "viewport", "scroll-to", ($x | into string), ($y | into string)] --socket=$socket
}

export def "glorp scene ensure" [--socket: string = ""] {
  glorp-cli-json ["scene", "ensure"] --socket=$socket
}
