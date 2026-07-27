# proto

Wire-format definitions shared between firmware and the backend. Generation is
**local-only** for now — we do not push these to a buf.build module yet.

## Layout

```
proto/
├── buf.yaml                                # v2 lint + breaking-change config (STANDARD lint, FILE breaking)
└── flow_controller/v1/
    └── valve.proto                         # ValveState enum + Uplink / Downlink messages
```

The directory structure follows buf conventions
(`<module>/<version>/<file>.proto`), which keeps the linter happy
(`PACKAGE_DIRECTORY_MATCH`) and means we can lift this into a buf.build module
later with no restructure.

## Workflow

```bash
mise run proto:lint         # buf lint
mise run proto:format       # buf format -w (writes in place)
mise run proto:gen          # regenerate Rust types into domain/src/proto/
mise run proto:check        # run :gen and verify nothing changed (CI)
```

After editing any `.proto`, run `mise run proto:gen` and **commit both** the
`.proto` change and the regenerated Rust file.

## Why no `buf.gen.yaml`?

`micropb` (the Rust runtime) and `micropb-gen` (the codegen library) do not
ship a `protoc-gen-micropb` plugin. `micropb-gen` is invoked directly from
Rust via the `proto_gen` workspace crate; `buf` only handles lint, format,
and breaking-change checks here.

If we later add a target that *does* use `buf generate` (e.g. a host backend
in another language), we'll add `buf.gen.yaml` then.

## Schema notes

- **Single shared `ValveState` enum** — `UNSPECIFIED (0)`, `OPEN (1)`,
  `CLOSED (2)`. The backend sends `OPEN` or `CLOSED` as `desired_state`; an
  unset/`UNSPECIFIED` value is ignored. There is no dedicated "unknown" wire
  value — the firmware's internal `Unknown` (erased flash / no known position)
  maps to `UNSPECIFIED` and is resolved to `CLOSED` at boot.
- **`current_state` vs. `last_commanded_state`.** Echoing the most recent
  command back lets the backend detect dropped downlinks (no echo within N
  uplinks → resend) without confirmed-downlink semantics.

## Deferred

- **buf.build module publish.** When the backend repo also consumes these
  protos in earnest, push `flow_controller/v1` to a buf.build module and have
  both sides depend on it via `buf.lock`.
