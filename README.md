# netoolfy

**All-in-one network diagnostic tool with automatic diagnosis.**

> ⚠️ **Work in progress** — this project is under active development.

## What is this?

`netoolfy` doesn't just show you raw numbers like `ping` or `dig`. It tells you **where the problem is** in plain language:

```
✔ Interface   wlan0 up, IP 192.168.1.23, signal good
✔ Gateway     192.168.1.1 reachable, 2.1 ms
✘ DNS         ISP resolver slow (240 ms) vs 1.1.1.1 (18 ms)
✔ Internet    0% loss, jitter 3 ms

Verdict: Your connection is fine, but your ISP's DNS is slow.
Fix: switch your DNS to 1.1.1.1 or 8.8.8.8.
```

## Privacy & Safety

- **No telemetry.** No network calls except the probes you ask for.
- **Not a scanner.** No port scanning, no flooding, no spoofing.
- Only probes: your local interface/gateway, well-known public resolvers, and targets you explicitly pass.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
