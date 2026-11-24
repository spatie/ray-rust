---
title: Internal protocol comparison
weight: 6
---

The reference is [spatie/ray at dd0a280](https://github.com/spatie/ray/tree/dd0a28063c15e847f5d67a5855c97c273ddfe693), together with the [framework-agnostic PHP guide](https://myray.app/docs/php/vanilla-php/usage). This port uses the same HTTP request envelope and Ray payload types. It does not wrap or require PHP.

## Implemented and exercised

- **`ray`, multiple values, updates:** `ray!`, `send`, `send_many`. Verification: Captured PHP HTTP comparison; stable UUID and callsite tests.
- **Strings, integers, bool, null, large integers:** `send`. Verification: Captured PHP comparison, including false, zero, empty string and null.
- **Colors, sizes, labels, collapse/remove:** Snake-case methods, `color`, `size`, `label`, `hide`, `remove`. Verification: Captured PHP comparison.
- **Named/new/clear screens and screen colors:** `new_screen`, `clear_screen`, `clear_all`, `screen_color` and shortcuts. Verification: Captured PHP comparison.
- **Notifications, confetti, separators, show/hide app:** `notify`, `confetti`, `separator`, `show_app`, `hide_app`. Verification: Captured PHP comparison.
- **HTML, literal text, custom content:** `html`, `text`, `send_custom`. Verification: Captured PHP comparison.
- **Links:** `link`, `url`. Verification: Compared against the equivalent PHP HTML payload; Rust escapes attributes and labels.
- **Tables with primitive cells:** `table`, `table_with_label`. Verification: Captured PHP comparison.
- **JSON representation:** `to_json`. Verification: Captured PHP comparison, including Unicode and slash escaping.
- **Image URLs, base64/data URIs:** `image`. Verification: Captured PHP comparison.
- **UTF-8 files and missing-file messages:** `file`. Verification: Captured PHP comparison.
- **Simple formatted XML:** `xml`. Verification: Captured PHP comparison of nested and empty elements.
- **Expansion controls:** `expand`, `expand_keys`, `expand_all`. Verification: Captured PHP comparison of commands; actual expansion depends on the app's renderer.
- **Raw scalar values:** `raw`. Verification: Captured PHP comparison.
- **Counters and counter identity:** `count`, `count_named`, `counter_value`, `clear_counters`. Verification: Captured PHP comparison and independent-name/reset tests.
- **Conditional sending and removal:** `when`, `when_with`, `show_if`, `remove_when`. Verification: Captured PHP comparison and gate/callback tests.
- **Enable/disable:** `enable`, `disable`, `enabled`, `disabled`. Verification: Captured PHP comparison and isolated-client tests.
- **Per-callsite limits and once:** `limit`, `once`. Verification: Captured PHP comparison and zero/negative/independent-callsite tests.
- **Global and per-second rate limits:** `Client::max`, `per_second`, `clear_rate_limits`. Verification: Captured PHP total-limit comparison and sliding-window/recovery tests.
- **Timing:** `measure`, `measure_with`, `stop_time`. Verification: HTTP tests of new timer, lap, positive elapsed time and reset.
- **Caller, trace and backtrace:** `caller`, `trace`, `backtrace`, explicit `Frame` methods. Verification: HTTP tests of Rust symbols, frame schema and mapped paths.
- **Error display:** `exception`. Verification: HTTP tests of message, frames and red styling.
- **Pause/Continue/Stop:** `pause`. Verification: Fake-server tests of Continue, Stop and lock query metadata.
- **App queries:** `Client::windows`, `theme`. Verification: HTTP query/error tests.
- **Custom payloads:** `Payload::new`, `send_request`. Verification: HTTP extension-payload test.
- **Returning values:** `pass`. Verification: Executed rustdoc assertion.

The reproducible comparison in `tests/protocol/compare.py` runs the real PHP and Rust clients against local HTTP capture servers. It compares payload content and UUID relationships, removing only runtime-specific origin and language/version metadata. `tests/fixtures/php-wire.json` stores the PHP capture for normal Rust-only CI. Run the comparison script with `--help` for fixture-generation options. See [testing](docs/testing.md) for Rust checks.

These checks establish wire compatibility, not desktop rendering or a real user's interaction with the Ray app. The desktop app has not been exercised by this test suite.

## Deliberate Rust differences

- Rust values use `serde::Serialize`. Structs, arrays, maps and floating-point values use `json_string`, Ray's native JSON renderer. PHP's `send` uses Symfony VarDumper HTML for these types. Values are preserved, but their presentation and clipboard data are not a byte-for-byte PHP reflection dump.
- `json` parses and displays native JSON instead of producing Symfony HTML. Invalid JSON returns `Error::Json`; PHP displays null. `raw` and nested table cells use escaped, preformatted JSON for structured values. `debug` accepts `Debug`-only types.
- Rust has no variadic methods or default arguments. Use `send_many`, `table_with_label`, `expand_keys`, `count_named`, `measure_with` and `when_with`; pass `""` for an optional screen/timer/link label. Camel-case PHP methods become snake_case. Deprecated spelling aliases are generally omitted.
- Timers use a monotonic Rust clock. PHP heap-memory statistics are omitted; their required wire fields are zero. Rust errors have observation-site backtraces rather than throw-site snippets. Symbols may be absent in stripped binaries.
- Client clones share session state. Independent clients do not. The default macro has one process-wide client; constructing an explicit `Client` avoids hidden configuration. Environment variables are loaded only with `Config::from_env()`.
- All sending methods return `Result<Ray, Error>` and consume the handle. Network errors are ignored by default, as in PHP; `throws(true)` reports them. Serialization, state, XML and non-missing-file errors propagate. App queries always report errors. `pause` returns an error for Stop instead of killing the process.
- Remote path replacement is a prefix mapping. Link text/attributes are escaped. File input must be UTF-8. XML uses a Rust parser, so complex-document formatting, entity spellings and declarations can differ from PHP DOM output.
- The HTTP availability probe/cache is not copied. A send directly attempts the request, with the same two-second default timeout. Redirects and proxies are disabled. The default client does not load PHP settings files.

## Intentionally omitted

Framework-specific adapters, service providers, model/query/mail/event integrations and CMS plugins are outside the framework-agnostic package scope. No framework runtime is emulated.

PHP-only features are omitted: `phpinfo` and INI inspection, Carbon-specific introspection, `invade` private-property/method access, PHP closure-as-value execution, deferred exception queues and shutdown rethrows, `die`/`rd` process termination, runtime method registration, PHPStan/Rector rules and the PHP source-removal script. Use normal Rust closures, `Result`, extension traits and source tooling instead.

PHP request hooks are not reproduced as global mutable callbacks. Wrap an explicit client call or construct custom protocol payloads. Rust does not silently catch panics. The undocumented `ban` and `charles` convenience messages can be expressed as ordinary `send` calls.

## Community clients

Existing third-party Rust clients include [bnomei/ray-dbg](https://github.com/bnomei/ray-dbg), also linked from the Ray site, and [larsmbergvall/ray-debug](https://github.com/larsmbergvall/ray-debug). This crate is a separate port of Spatie's PHP client.
