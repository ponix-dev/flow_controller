# Phase 2 Plan — `proto/` + buf + micropb codegen

## Context

ROADMAP.md Phase 2 establishes the wire format the firmware and backend will share. The end state of this phase: `.proto` definitions live in-tree, `buf` lints them, and `micropb`-generated Rust types are usable from the firmware. This phase does **not** yet wire those types into the LoRaWAN uplink/downlink path — that's Phase 3.

A key research finding reshapes the original ROADMAP.md sketch: **`buf` does NOT orchestrate `micropb` codegen**. `micropb-gen` is a Rust build-time crate that calls `protoc` internally; there is no `protoc-gen-micropb` plugin that `buf.gen.yaml` could invoke. So `buf`'s role here is lint / format / breaking-change checks only, and codegen lives in Rust.

User decisions taken:

- **Schema shape**: single `ValveState` enum reused by `Uplink` and `Downlink`. Backend doesn't send `UNKNOWN` as a command.
- **Codegen flow**: pre-generated via a `mise` task; generated `.rs` is committed. `cargo build` does not need `protoc`.

## Approach

### Generator architecture

Add a new host-only workspace crate `proto_gen` whose only job is to call `micropb_gen::Generator` and write Rust files into `firmware/src/proto/`. A `mise run proto:gen` task invokes it via `cargo run -p proto_gen`. Developer workflow:

1. Edit `proto/flow_controller.proto`.
2. Run `mise run proto:gen`.
3. Commit both the `.proto` and the regenerated `.rs`.

CI that "just builds firmware" needs no `protoc`. CI that verifies proto is up-to-date runs `mise run proto:gen` and checks `git status` is clean.

### `buf` role

`buf.yaml` (v2) configures lint and breaking checks. **No `buf.gen.yaml`** — buf isn't generating anything. `mise` exposes `proto:lint` and `proto:format` tasks for ergonomics.

### Schema

Single `.proto` file at `proto/flow_controller.proto`, package `flow_controller.v1` (room to grow without restructure if we later push to buf.build):

```proto
syntax = "proto3";
package flow_controller.v1;

enum ValveState {
  VALVE_STATE_UNSPECIFIED = 0;
  VALVE_STATE_OPEN = 1;
  VALVE_STATE_CLOSED = 2;
  VALVE_STATE_UNKNOWN = 3;
}

// Sent by backend on each Class-A poll.
message Downlink {
  ValveState desired_state = 1;
}

// Sent by device on each Class-A uplink.
message Uplink {
  ValveState current_state = 1;
  ValveState last_commanded_state = 2;
}
```

### `no_std` compatibility

`micropb-gen` defaults can pull `alloc`-based containers. These messages have only enum-typed fields (no strings, no `repeated`), so the default code should be `no_std`-clean. The `proto_gen` binary will explicitly set `Config` to fixed/heapless container types as a defensive measure even though no such fields exist yet.

## Files to add

| Path | Purpose |
|---|---|
| `proto/buf.yaml` | v2 config; lint: STANDARD, breaking: FILE |
| `proto/flow_controller.proto` | The schema above |
| `proto/README.md` | Explain layout, regen workflow, why no `buf.gen.yaml`, deferred buf.build push |
| `proto_gen/Cargo.toml` | Host crate (`edition = "2021"`, no `no_std`); deps: `micropb-gen` |
| `proto_gen/src/main.rs` | ~30 lines: parse `--out` arg, invoke `micropb_gen::Generator::compile_protos` |
| `firmware/src/proto/mod.rs` | `pub mod flow_controller;` (or `include!` of generated file, depending on micropb-gen's output convention) |
| `firmware/src/proto/flow_controller.rs` | **Generated** by `mise run proto:gen`; committed |

## Files to modify

| Path | Change |
|---|---|
| `/Users/srall/development/flow_controller/Cargo.toml` | Add `proto_gen` to `[workspace] members`; consider adding to `default-members` so root `cargo check` covers it |
| `/Users/srall/development/flow_controller/.mise.toml` | Add `[tools.buf]` (aqua backend) and `[tools.protoc]` (aqua: `protocolbuffers/protobuf`); add `[tasks.proto:gen]` (`cargo run -p proto_gen`), `[tasks.proto:lint]` (`buf lint proto`), `[tasks.proto:format]` (`buf format -w proto`), `[tasks.proto:check]` (verify clean regen — runs `proto:gen` then `git diff --exit-code -- firmware/src/proto`) |
| `/Users/srall/development/flow_controller/firmware/Cargo.toml` | Add `micropb` runtime dep with appropriate features (`["encode","decode","container-heapless"]` or similar; finalize once we read the latest `micropb` README) |
| `/Users/srall/development/flow_controller/firmware/src/main.rs` | Add `mod proto;` to declare the new module |

## Critical files to reference (read before/during implementation)

- `/Users/srall/development/flow_controller/Cargo.toml` — workspace config; mirror existing `[patch.crates-io]` style
- `/Users/srall/development/flow_controller/firmware/Cargo.toml` — match existing dep style (defmt-feature flags, version pinning)
- `/Users/srall/development/flow_controller/firmware/build.rs` — confirm no overlap with codegen (we're NOT modifying this file under the chosen pre-generation flow)
- `/Users/srall/development/flow_controller/.mise.toml` — mirror `[tasks.X]` style; `[tools."github:..."]` style for tools that need GitHub-release-asset patterns vs. simpler `[tools.buf]` for aqua-backed tools
- `/Users/srall/development/flow_controller/.cargo/config.toml` — confirm target / runner config doesn't change
- `/Users/srall/development/flow_controller/firmware/src/main.rs` — find a place for `mod proto;`
- `/Users/srall/development/flow_controller/CLAUDE.md` — update Project Structure to mention `proto/` and `proto_gen/`
- `/Users/srall/development/flow_controller/README.md` — update workspace layout listing
- `/Users/srall/development/flow_controller/ROADMAP.md` — check off Phase 2 items at the end

## Verification

End-to-end pass:

1. `mise install` succeeds and pulls `buf` + `protoc` at pinned versions.
2. `mise run proto:lint` passes (STANDARD lint rules satisfied).
3. `mise run proto:gen` produces `firmware/src/proto/flow_controller.rs` with no errors. The file contains `pub enum ValveState`, `pub struct Uplink`, `pub struct Downlink` (or whatever shape `micropb-gen` produces — confirmed by inspection).
4. `mise run proto:check` is a no-op when run a second time — confirms reproducibility.
5. `cargo check -p flow_controller --target thumbv7em-none-eabi` succeeds. **Critical**: the generated code must be `no_std`-clean. If `micropb-gen` defaults pull `alloc`, override via `Config` and regenerate.
6. `mise run build` (release) succeeds. Compare `.text` size before/after — should grow by a few hundred bytes for the codecs, no more.
7. `cargo check -p proto_gen` (host) succeeds.
8. `cargo check -p lorawan_flash` still succeeds (verifying we didn't break the host CLI workspace member).

Sanity:

- `git status` is clean after `mise run proto:gen` is run twice in a row.
- The generated `firmware/src/proto/flow_controller.rs` is committed; `firmware/src/proto/mod.rs` references it.
- `firmware/src/main.rs` adds `mod proto;` but does **not** yet call any of the generated types — Phase 3 wires them in.

## Out of scope (deferred)

- **Wiring `Uplink` / `Downlink` into the LoRaWAN path** — Phase 3.
- **Pushing the schema to a buf.build module** — explicitly local-only this round (per Open Questions in ROADMAP.md). When the backend repo also consumes these messages, deduplicate via a buf.build module then.
- **Backend interop / decoder verification** — out of scope.

## Open items the implementation may surface

- `micropb-gen` API may have changed since the agent's research; verify against the latest crates.io / GitHub README during implementation.
- The exact feature flags on `micropb` runtime (`container-heapless` vs. `container-arrayvec`) need to be picked once we read the current README. Default to whatever's idiomatic; we can change later when fields actually use containers.
- `protoc` is in the mise registry under `protoc` (aqua-backed); if it's not, fall back to `[tools."aqua:protocolbuffers/protobuf"]`.
- The generator may want a single output file per `.proto` or per `package` — confirm and reflect that in `firmware/src/proto/mod.rs`.

## Branch & roadmap bookkeeping

- Branch: `feat/proto-foundation` off `main`.
- After implementation lands, mark all four Phase 2 bullets in `ROADMAP.md` as complete with corrected wording (the original bullets reference `buf.gen.yaml` which we won't be using; rewrite to reflect actual decisions).
- Update `handoff.md` to set Phase 2 → done, Phase 3 → next.
