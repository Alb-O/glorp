# Transport

Glorp ships two stable host access paths:

- in-process: `glorp_transport::LocalClient`
- IPC: `glorp_transport::IpcClient` and `glorp_transport::start_server`

IPC uses one-request/one-response JSON messages over a local Unix socket.
The protocol carries the same command, query, and event types defined in `glorp_api`.

The stable repo-local socket path is `glorp.sock` at the repo root. `glorp_gui`
hosts or joins that socket, and the Nu/plugin surface auto-attaches to it when present.
When no shared runtime is live, the plugin starts `glorp_host` for that repo root.

The acceptance suite proves parity across:

- direct in-process host
- IPC client
- Nu plugin
- shared host auto-start
