# Azure Core — Technical Documentation

**Language:** Rust · **Dependencies:** `libc` only

---

## Overview

Azure Core is the shared base of every Azure crate: the data models the
daemons and apps agree on, the binary wire format, hand-written crypto, and
the security layer (identity, sandbox, daemon hardening, encrypted files).
It has no daemon and no UI of its own.

Built with `opt-level = 3` even in dev (see the workspace `Cargo.toml`): the
crypto and fingerprinting are too slow unoptimised.

## Layout

```
azure-core/src/
├── lib.rs
├── paths.rs               where daemon sockets live (private per-user directory)
├── daemon.rs              bind + serve: what every daemon does to open its socket
├── models/
│   ├── wire.rs            Writer / Reader : little-endian ints, u32-prefixed bytes and strings
│   ├── frame.rs           daemon frames (u32 size + content), status byte, read/write/check
│   ├── storage_model.rs   storage rules shared by azure-stockage and apps (limits, access)
│   └── window_model.rs    WindowSpec / WindowScope / WindowKind (router window sharing)
├── rules/
│   ├── window_event.rs    WindowEvent (close, resize, keys, mouse, scroll)
│   ├── window_provider.rs AzureWindowProvider (implemented by azure-engine's Window)
│   └── root_provider.rs   AzureRouterProvider (implemented by azure-rooter's IntraRouter)
├── managers/
│   └── identity.rs        who is on the other end of a unix socket
├── crypto/                SHA-256, HMAC, ChaCha20, Poly1305, AEAD, random (no dependency)
└── security/
    ├── sandbox.rs         Landlock sandbox for apps
    ├── hardening.rs       daemon hardening, private directories
    ├── vault.rs           encrypted files for daemons
    ├── registry.rs        ask azure-manager "is this process really app N?"
    ├── isolation.rs       namespaces set up by the launcher (older kernels)
    └── limits.rs          cap on simultaneous connections per daemon
```

## Socket paths (`paths.rs`)

`socket("manager")` → `$AZURE_RUNTIME_DIR/manager.sock`, else
`$XDG_RUNTIME_DIR/azure/manager.sock`, else `/tmp/azure-<uid>/manager.sock`.
Daemons call `prepare_socket_dir(socket)` before `bind`: the directory is
created with mode 700 and refused if another account owns it or others can
write to it.

## Daemons (`daemon.rs`, `models/frame.rs`)

`daemon::bind(socket)` prepares the directory, removes a socket left by a
stopped daemon and refuses one a live daemon still answers on;
`daemon::serve(listener, handler)` admits each client through the
`ConnectionGate` and runs `handler(stream)` in its own thread. Every daemon
(rooter, stockage, provider, service, manager) starts this way.

`frame::{read_frame(stream, max), write_frame, response, error,
check_status}`: requests and replies of stockage, provider, service and
manager. Each daemon keeps its own maximum frame size. The rooter's
requests are not framed (opcode then fields, see
`azure-rooter/src/models/request.rs`), but what it delivers is a frame.

## Identity (`managers/identity.rs`)

| Function | Returns |
|---|---|
| `peer_pid(stream)` | pid of the peer (`SO_PEERCRED`) |
| `peer_exe(stream)` | its executable (`/proc/<pid>/exe`) |
| `peer_fingerprint(stream)` | SHA-256 of the running binary |
| `peer_restricted(stream)` | `true` if any thread of the peer has `NoNewPrivs` (every sandboxed app) — or if `/proc` cannot tell |
| `fingerprint_file(path)`, `hex(bytes)` | helpers |

## Security (`security/`)

- **`sandbox.rs`** — `Sandbox::system()` then `.read(p)`, `.write(p)`,
  `.socket(p)`, `.network(bool)`, `.apply()` (or `.prepare()` + `enforce()`
  between `fork` and `exec`). What is enforced depends on the kernel's
  Landlock ABI (`abi_version()`):

  | ABI | Adds |
  |---|---|
  | 1–3 | files (read / write / execute), `REFER`, `TRUNCATE` |
  | 4 | TCP bind/connect denied unless `.network(true)` |
  | 5 | device ioctl |
  | 6 | scopes: no signals to processes outside the sandbox, no abstract unix sockets outside it |
  | 9 | named unix sockets: only the Azure socket directory (`paths::runtime_dir`), the Wayland socket and `.socket(p)` paths — no D-Bus, no X11 |

  `apply()` returns `Enforcement::{Full, FilesOnly, Unsupported}`;
  `is_sandboxed()` tells a process it is enclosed (it must not launch
  daemons, they would be enclosed with it).

  `/run` is read-only inside the sandbox; `/tmp`, `/var/tmp` and `/dev` are
  writable.

- **`isolation.rs`** — `Isolation::for_app(network)` then `enter()` inside
  `CommandExt::pre_exec`: new user, mount, PID (and network, without the
  network permission) namespaces. `$XDG_RUNTIME_DIR` becomes an empty tmpfs
  with only the Wayland socket and the Azure socket directory bound back;
  `/tmp/.X11-unix` and `/run/dbus` are hidden; the app becomes PID 1 of its
  namespace with its own `/proc`. Used by `azure run` for every installed
  app, and by `AzureApp` to relaunch an installed app started directly on a
  kernel without Landlock ABI 9. If unprivileged namespaces are forbidden,
  `enter` does nothing (Landlock still applies).
- **`limits.rs`** — `ConnectionGate::default().admit(&stream)`: at most 64
  simultaneous connections per client process and 1024 in total; the
  returned `Permit` frees its slot when dropped.
- **`hardening.rs`** — `harden_daemon()` (`PR_SET_DUMPABLE 0`, `umask 077`),
  `private_dir(path)` (mode 700), `is_dumpable()`.
- **`vault.rs`** — `Vault::open(dir)`: ChaCha20-Poly1305, key in
  `dir/cle.bin` (0600), files tagged `AZS1` + a label; plaintext files from
  before encryption are still read.
- **`registry.rs`** — `verify(manager_socket, id, stream)`: for ids ≥ 1000,
  asks azure-manager (opcode `IDENTIFY`) and checks the peer is that app
  (same fingerprint when installed, same executable or fingerprint in
  development). `Ok(None)` when the manager does not know the id or is
  unreachable: the daemon then applies its own first-come rule.

## Crypto (`crypto/`)

Written by hand, checked against the official test vectors
(`azure-stockage/tests/crypto_vectors.rs`) and against an independent
implementation at the edge cases (`azure-stockage/tests/crypto_oracle.rs`).
Tag comparison is constant-time; randomness comes from `getrandom`. Not
audited by an outside party.

## Tests

`cargo test -p azure-core` — `tests/security.rs` runs each sandbox case in
a fresh process (Landlock is irreversible): files, network, signals, named
sockets, the real session bus and namespace isolation; `tests/paths.rs`
and `tests/limits.rs` cover the socket directory and connection caps.
