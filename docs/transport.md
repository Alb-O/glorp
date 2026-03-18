# Transport

Glorp ships two stable host access paths:

- in-process: `glorp_transport::LocalClient`
- IPC: `glorp_transport::IpcClient` and `glorp_transport::start_server`

IPC uses one-request/one-response JSON messages over a local Unix socket.
The protocol carries the same command, query, and event types defined in `glorp_api`.

The stable repo-local socket path is `glorp.sock` at the repo root. `glorp_gui`
hosts or joins that socket, and `glorp_cli` auto-attaches to it when present.

The acceptance suite proves parity across:

- direct in-process host
- IPC client
- Nu plugin
- CLI
