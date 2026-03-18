# Transport

Glorp ships two stable host access paths:

- in-process: `glorp_transport::LocalClient`
- IPC: `glorp_transport::IpcClient` and `glorp_transport::start_server`

IPC uses one-request/one-response JSON messages over a local Unix socket.
The protocol carries the same command, query, and event types defined in `glorp_api`.

The acceptance suite proves parity across:

- direct in-process host
- IPC client
- Nu plugin
- CLI
