# CLAUDE.md — netoolfy

> Working name: `netoolfy` (check crates.io / GitHub for name availability in Phase 0 and rename if taken).
> This file is the source of truth for the project. Read it fully at the start of every session.

## 1. What we are building

`netoolfy` is an all-in-one network diagnostic tool written in Rust (CLI first, TUI later).
Its differentiator is **automatic diagnosis**: it does not just print raw data like `ping`/`dig`/`traceroute`,
it tells the user **where the problem is** in plain language.

Example verdict:

```
✔ Interface   wlan0 up, IP 192.168.1.23, signal good
✔ Gateway     192.168.1.1 reachable, 2.1 ms
✘ DNS         ISP resolver slow (240 ms) vs 1.1.1.1 (18 ms)
✔ Internet    0% loss, jitter 3 ms

Verdict: Your connection is fine, but your ISP's DNS is slow.
Fix: switch your DNS to 1.1.1.1 or 8.8.8.8.
```

Target user: someone who asks "why is my internet slow?" and is not a networking expert.
Primary goal: a polished, useful open-source project that earns GitHub stars.

## 2. Non-negotiable principles

1. **Diagnosis over data.** Every feature should feed the verdict engine or make its output clearer.
2. **Layered checks.** Diagnose bottom-up: interface → gateway → DNS → internet → specific target. A failure at a lower layer explains failures above it (do not report 5 symptoms of 1 cause).
3. **Single static binary**, no runtime dependencies, fast startup (<100 ms for help, <5 s for a full diagnosis by default).
4. **Cross-platform by design:** Linux first, then macOS and Windows. Platform-specific code lives behind traits in `platform/`.
5. **Testable without a network.** All I/O goes through traits so checks and the verdict engine can be unit-tested with mocks.
6. **Safe by default.** See section 6.

## 3. Tech stack

- Language: Rust, stable toolchain, edition 2021 or newer, MSRV documented in `Cargo.toml`.
- Async: `tokio`
- CLI: `clap` (derive)
- DNS: `hickory-resolver`
- ICMP: `surge-ping` (or `socket2`/`pnet` if needed; decide in Phase 5 and document why)
- TLS: `tokio-rustls` + `rustls-native-certs` or `webpki-roots`
- HTTP timing: hand-rolled on top of tokio/rustls for DNS/connect/TLS/TTFB breakdown (avoid heavy HTTP clients)
- Interface/gateway info: `netdev` or `default-net` (evaluate, pick one)
- Serialization: `serde`, `serde_json`
- Errors: `thiserror` in the library, `anyhow` only in `main.rs`
- Output: `owo-colors` or `colored`, `comfy-table` if tables are needed
- TUI (Phase 6): `ratatui` + `crossterm`
- Logging: `tracing` (+ `tracing-subscriber`), silent by default, `-v/-vv` to enable

**Rule:** never write crate versions from memory. Use `cargo add <crate>` so the latest compatible version is resolved, and
keep dependencies minimal. Justify every new dependency in the commit message.

## 4. Architecture

Library + binary split so the logic is reusable and testable.

```
netoolfy/
├─ Cargo.toml
├─ CLAUDE.md
├─ README.md
├─ LICENSE-MIT / LICENSE-APACHE
├─ src/
│  ├─ main.rs            # thin: parse args, call lib, set exit code
│  ├─ lib.rs
│  ├─ cli.rs             # clap definitions
│  ├─ config.rs          # defaults: targets, timeouts, thresholds
│  ├─ model.rs           # Status, Finding, CheckResult, Report, Verdict
│  ├─ checks/
│  │  ├─ mod.rs          # Check trait + runner
│  │  ├─ interface.rs
│  │  ├─ gateway.rs
│  │  ├─ dns.rs
│  │  ├─ internet.rs     # ping, loss, jitter
│  │  └─ target.rs       # port, TLS, HTTP timing
│  ├─ diagnosis.rs       # verdict engine (pure functions, heavily tested)
│  ├─ output/
│  │  ├─ mod.rs
│  │  ├─ pretty.rs       # colored human output
│  │  └─ json.rs
│  ├─ platform/          # OS-specific code behind traits
│  └─ tui/               # Phase 6
├─ tests/                # integration tests
└─ .github/workflows/    # CI + release
```

Core concepts:

- `trait Check { fn name(&self) -> &'static str; async fn run(&self, ctx: &Context) -> CheckResult; }`
- `Status`: `Ok | Warn | Fail | Skipped` (Skipped = a lower layer failed, so this check is meaningless)
- `Finding`: machine-readable code (e.g. `DnsSlow`), human message, optional suggested fix, raw metrics
- `Context`: holds config, injected resolvers/pingers (trait objects) so tests can mock them
- The verdict engine takes `Vec<CheckResult>` and returns a `Verdict` with a root cause and a recommended fix

## 5. How you must work (agent protocol)

- **One phase at a time.** Do exactly the phase marked `CURRENT` in section 8. Do not start the next phase.
- **Plan first.** At the start of a phase, write a short plan (files to create/change, key decisions) and then execute.
- **Small, reviewable commits.** One logical change per commit, conventional commits (`feat:`, `fix:`, `test:`, `docs:`, `chore:`).
- **Quality gate before declaring a phase done.** All must pass:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo build --release`
- **No `unwrap()`/`expect()` in library code** except where an invariant is proven and commented. No `unsafe` unless unavoidable and documented.
- **Every public item gets a doc comment.** Every check gets unit tests with mocks.
- **If something is ambiguous, ask me** instead of guessing. If you must assume, write the assumption in the phase report.
- **End each phase with a report:** what was done, what was tested, known limitations, and a proposal for the next phase.
  Then update the checklist in section 8 (mark the phase done, move `CURRENT` forward) and stop.
- Do not refactor unrelated code or rename things without telling me why.
- Do not claim something works if you did not run it. State clearly what you could and could not verify (e.g. "not tested on Windows").

## 6. Safety and ethics rules

`netoolfy` is a diagnostic tool, not a scanner or attack tool.

- Only probe: the local interface/gateway, well-known public resolvers and anchors (configurable), and targets the user explicitly passes.
- **No port-range scanning, no flooding, no spoofing.** `netoolfy port` checks one explicitly given host:port at a time.
- Rate limit probes, always use timeouts, cap retries.
- Default public targets must be neutral and documented (e.g. 1.1.1.1, 8.8.8.8, 9.9.9.9). Make them overridable.
- No telemetry. No network calls except the probes the user asked for. State this in the README.
- Never log or store secrets. History files (Phase 7) store only metrics and verdicts.

## 7. Conventions

- Rust style: idiomatic, small functions, explicit error types, no stringly-typed errors in the library.
- Timeouts and thresholds live in `config.rs` as named constants (e.g. `DNS_SLOW_MS`), never magic numbers inline.
- Exit codes: `0` healthy, `1` warnings, `2` failures, `64+` usage/internal errors. Document in README.
- Output must degrade gracefully: respect `NO_COLOR`, detect non-TTY, and keep `--json` output stable and documented.
- English for code, comments, docs, commit messages, and README (for GitHub reach).

## 8. Roadmap and progress tracker

Status legend: `[ ]` todo, `[~]` CURRENT, `[x]` done. **Only work on the `[~]` phase.**

### [x] Phase 0 — Project scaffolding
Goal: a clean, buildable repo with tooling and CI.
- `cargo new` with lib + bin layout from section 4 (empty modules with doc comments)
- Choose license (MIT OR Apache-2.0), add `LICENSE` files, `.gitignore`, `rustfmt.toml`, `clippy` config
- Set up `Cargo.toml` metadata (description, repository, keywords, categories, rust-version)
- Add GitHub Actions: fmt + clippy + test on Linux/macOS/Windows
- Write a minimal README (name, one-line pitch, "work in progress")
- Check name availability on crates.io and GitHub; report result
Done when: CI is green on all three OSes with an empty skeleton.

### [x] Phase 1 — Core model and check framework
Goal: the foundation every check will use.
- Implement `model.rs` (`Status`, `Finding`, `CheckResult`, `Report`, `Verdict`) with `serde` support
- Implement the `Check` trait, `Context`, and a runner that executes checks layer by layer and marks upper layers `Skipped` when a lower layer fails
- Implement `config.rs` with default targets, timeouts, thresholds
- Add a trivial dummy check to prove the runner works
- Unit tests for the runner and skip logic
Done when: runner passes tests with mocked checks, including the "lower layer fails → upper layers skipped" case.

### [~] Phase 2 — Interface and gateway checks
Goal: layers 1 and 2.
- Interface check: find active interface, detect IP presence, link up/down, (Linux) WiFi signal if available
- Gateway check: detect default gateway, ping it, measure latency and loss
- Platform abstraction trait in `platform/`, Linux implementation first, stubs for macOS/Windows that return `Skipped` with a clear message
- Unit tests with mocked platform info
Done when: `netoolfy` on Linux reports interface and gateway status correctly; failures produce clear findings.

### [ ] Phase 3 — DNS check
Goal: layer 3, the most valuable differentiator.
- Detect the system resolver(s) and resolve a set of test domains
- Compare system DNS vs 1.1.1.1 vs 8.8.8.8 (latency, success, consistency)
- Findings: `DnsSlow`, `DnsFailing`, `DnsHijackSuspected` (different answers between resolvers; phrase carefully, it is only a hint)
- Suggested fix text for each finding
- Tests with a mocked resolver
Done when: a slow or broken system resolver is correctly identified while public resolvers work.

### [ ] Phase 4 — Internet check (ping, loss, jitter)
Goal: layer 4.
- Send N ICMP echo requests to several targets, compute min/avg/max, loss %, jitter
- Thresholds from `config.rs` → `Ok/Warn/Fail`
- Handle missing ICMP privileges gracefully: fall back to TCP connect timing and say so in the output
- Tests for the statistics math (edge cases: 0 replies, 1 reply, all replies)
Done when: loss/jitter are computed correctly and the privilege fallback works.

### [ ] Phase 5 — Verdict engine and CLI polish
Goal: the feature that makes the project special.
- Implement `diagnosis.rs`: map combinations of results to a root cause + plain-language verdict + fix
- Cover at least 10 scenarios in table-driven tests (all good, WiFi weak, gateway down, DNS slow, DNS dead but IP works, high loss, high jitter, ISP outage, etc.)
- Pretty output (`output/pretty.rs`) with colors/symbols, `--json`, `--no-color`, exit codes
- Subcommands: `netoolfy` (full), `netoolfy ping <t>`, `netoolfy dns <domain>`, `netoolfy port <host:port>`
- `--help` text and examples are clear
Done when: a full run prints a correct verdict in all tested scenarios and JSON output is stable.

### [ ] Phase 6 — Target checks (port, TLS, HTTP timing)
Goal: layer 5.
- TCP port check with timeout
- TLS check: handshake time, certificate expiry (warn at 14 days), chain validity, SAN match
- HTTP timing breakdown: DNS / connect / TLS / TTFB / total
- Integrate into the verdict ("your network is fine, the problem is the server")
Done when: `netoolfy https://example.com` shows a timing breakdown and cert info and feeds the verdict.

### [ ] Phase 7 — Cross-platform hardening
Goal: real support for macOS and Windows.
- Implement platform traits for macOS and Windows (interface, gateway, wifi if feasible)
- Resolve the ICMP privilege story per OS and document it
- CI matrix runs the integration tests on all three OSes
- Document known limitations per OS in README
Done when: the full diagnosis works on all three OSes (or limitations are documented honestly).

### [ ] Phase 8 — TUI live mode
Goal: the "wow" factor for the README GIF.
- `netoolfy --live` / `netoolfy tui` using `ratatui` + `crossterm`
- Live latency sparkline/graph per layer, color-coded status, verdict panel that updates in real time
- Keyboard controls (`q` quit, `p` pause, `r` reset), resize handling, clean terminal restore on panic
- Keep the TUI layer thin: it only renders data from the same library
Done when: the TUI runs smoothly for 10+ minutes without leaks or flicker.

### [ ] Phase 9 — Advanced features
Pick in this order, each as its own sub-phase with its own report:
1. Traceroute/MTR mode with per-hop stats (and ASN/geo only if it can be done without telemetry-like third-party calls, otherwise offline DB, else skip)
2. History: store results locally (JSON/SQLite), `netoolfy history`, `netoolfy compare` ("yesterday vs now")
3. MTU discovery and IPv6 checks
4. Optional Prometheus exporter mode

### [ ] Phase 10 — Release and launch
Goal: make it easy to install and impossible to ignore.
- README: pitch, demo GIF (record with `vhs` or `asciinema`), install one-liner, usage examples, exit codes, comparison with `ping/mtr/trippy`, safety/privacy statement
- GitHub Actions release workflow: prebuilt binaries for Linux (gnu + musl), macOS (x86_64 + aarch64), Windows
- Publish to crates.io; prepare packaging files for Homebrew, AUR, Nix
- `CONTRIBUTING.md`, issue templates, `CHANGELOG.md`, semantic version `0.1.0`
- Prepare launch posts (r/rust, Show HN, r/selfhosted, Rust Indonesia community) as drafts for me to review
Done when: `cargo install netoolfy` and a binary release both work from a clean machine.

## 9. Definition of done (every phase)

- [ ] Quality gate from section 5 passes
- [ ] New logic has unit tests; edge cases covered
- [ ] Docs/README updated if user-facing behavior changed
- [ ] Phase report written
- [ ] Checklist in section 8 updated, `CURRENT` marker moved