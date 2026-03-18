export def "nu-complete glorp exec-op" [] { ["txn" "config-set" "config-reset" "config-patch" "config-reload" "config-persist" "document-replace" "editor-motion" "editor-mode" "editor-insert" "editor-backspace" "editor-delete-forward" "editor-delete-selection" "editor-history"] }
export def "nu-complete glorp query-op" [] { ["schema" "config" "document-text" "editor" "capabilities"] }
export def "nu-complete glorp helper-op" [] { ["session-attach" "session-shutdown" "config-validate" "events-subscribe" "events-next" "events-unsubscribe"] }
