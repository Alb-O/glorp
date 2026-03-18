# Config

Durable config lives in ../nu/default-config.nu.

Rules:

- Config is data-first Nu, not arbitrary runtime mutation.
- Public paths are stable and introspectable.
- Values use canonical kebab-case enum tokens.
- `config set`, `config patch`, and `config reset` mutate effective runtime config.
- `config persist` writes the current effective config back to the durable Nu file.
- `config reload` discards in-memory config mutations and reloads the durable file.
