# Transport

Glorp ships two stable host access paths:

- in-process: `glorp-transport::LocalClient`
- IPC: `glorp-transport::IpcClient` and `glorp-transport::start_server`

IPC uses one-request/one-response JSON messages over a local Unix socket.
The protocol carries the same command, query, and event types defined in `glorp-api`.

The acceptance suite proves parity across:

- direct in-process host
- IPC client
- Nu plugin
- CLI
