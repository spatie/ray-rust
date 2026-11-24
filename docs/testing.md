---
title: Testing
weight: 7
---

Run these checks from the package directory:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets --all-features
cargo test --locked --doc --all-features
cargo doc --locked --no-deps --all-features
cargo package --locked
```

Set `RUSTDOCFLAGS` to `-D warnings` when running the documentation commands. In POSIX shells, prefix each command with `RUSTDOCFLAGS='-D warnings'`. In PowerShell, first run `$env:RUSTDOCFLAGS = '-D warnings'`.

README.md and all Markdown pages in docs/ participate in rustdoc tests. Setup lines prefixed with `# ` compile and execute but are hidden in rendered rustdoc. Assertions check the examples' results.

Check the minimum supported Rust version:

```sh
rustup toolchain install 1.89.0 --profile minimal
cargo +1.89.0 test --locked --all-targets --all-features
cargo +1.89.0 test --locked --doc --all-features
```

Check security advisories:

```sh
cargo install cargo-audit --locked
cargo audit
```

CI runs stable checks on Linux, macOS and Windows, and the MSRV and audit checks on Linux. An audit requires access to the RustSec advisory database.

## PHP protocol comparison

The protocol CI job runs PHP 8.4 and checks out the pinned PHP reference. To run it locally, install PHP with curl, DOM and mbstring, Composer and Python 3, then:

```sh
git clone https://github.com/spatie/ray ../php-ray
git -C ../php-ray checkout dd0a28063c15e847f5d67a5855c97c273ddfe693
composer install --working-dir=../php-ray --no-dev --prefer-dist --no-interaction
python3 tests/protocol/compare.py --php-root ../php-ray
```

The script starts disposable HTTP capture servers, sends the same scenario through both real clients, and compares 60 requests containing 66 payloads. It checks the root POST endpoint, JSON content type, User-Agent, origin shape, payload content, project name and stable UUID relationships. Only language/version metadata and runtime-specific source locations are normalized. It does not claim desktop rendering parity.

The normal Rust tests compare the same scenario against `tests/fixtures/php-wire.json`, captured from PHP. After deliberately changing the reference or scenario, review the resulting diff before using `--write-fixture` to update it. No PHP installation is needed for ordinary `cargo test`.

Other HTTP tests cover isolated sessions, callsite metadata, nested serialization, limits, timer laps, errors, path mapping, pause/continue and Stop. The pause rustdoc example compiles only because it needs a person to press Continue; fake-server tests exercise that behavior automatically. Other guide examples execute with best-effort output, so no open desktop app is required.
