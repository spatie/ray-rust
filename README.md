# Send debug output from Rust to Ray

[![Latest version on crates.io](https://img.shields.io/crates/v/spatie-ray.svg?style=flat-square)](https://crates.io/crates/spatie-ray)
[![Tests](https://img.shields.io/github/actions/workflow/status/spatie/ray-rust/ci.yml?branch=main&style=flat-square)](https://github.com/spatie/ray-rust/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square)](https://github.com/spatie/ray-rust/blob/main/LICENSE.md)
[![Total downloads](https://img.shields.io/crates/d/spatie-ray.svg?style=flat-square)](https://crates.io/crates/spatie-ray)

This is the Rust port of [spatie/ray](https://github.com/spatie/ray). Send debug output from any Rust application to [Ray](https://myray.app), the desktop debugging app from [Spatie](https://spatie.be/).

Send values, HTML, JSON, XML, tables, files and images. Color and label entries, measure runtime, count calls, pause execution and control the app using the same protocol as the PHP client.

## Support us

[<img src="https://spatie.be/github-ad-image/ray-rust.jpg" alt="Support Spatie's open source work" width="419px" />](https://spatie.be/github-ad-click/ray-rust)

We invest a lot of resources into creating [best in class open source packages](https://spatie.be/open-source). You can support us by [buying one of our paid products](https://spatie.be/open-source/support-us).

We highly appreciate you sending us a postcard from your hometown, mentioning which of our package(s) you are using. You'll find our address on [our contact page](https://spatie.be/about-us). We publish all received postcards on [our virtual postcard wall](https://spatie.be/open-source/postcards).

## Installation

Requires Rust 1.89 or newer and the [Ray desktop app](https://myray.app).

```sh
cargo add spatie-ray
```

## Usage

```rust
# fn main() -> Result<(), spatie_ray::Error> {
use spatie_ray::{ray, json};

ray!("Hello world")?;
ray!(json!({"a": 1, "b": 2}))?.red()?;
ray!("John", "Paul", "George", "Ringo")?.label("Beatles")?;
# Ok(())
# }
```

`ray!` captures the call's file and line. `ray!()` starts an empty entry; `ray!(value, ...)` sends one payload per value and returns `Result<Ray, Error>`. Fluent updates keep the same UUID. Transport errors are ignored by default, so a closed Ray app does not break your application. Serialization errors are returned.

Structured values use Serde and Ray's native JSON renderer. Use `debug(&value)` for `Debug`-only types. See the [usage guide](https://github.com/spatie/ray-rust/blob/main/docs/usage.md) and [configuration](https://github.com/spatie/ray-rust/blob/main/docs/configuration.md).

The default connection is `localhost:23517`, with a two-second timeout. For another host, environment configuration, error reporting, or an isolated session:

```rust
# fn main() -> Result<(), spatie_ray::Error> {
use spatie_ray::{Client, Config, ray};

let client = Client::new(Config::from_env()?.project("My application"));
ray!(client: client, "Hello world")?.green()?;
# Ok(())
# }
```

## Testing

```sh
cargo test --locked --all-targets --all-features
cargo test --locked --doc --all-features
```

See [all quality checks](https://github.com/spatie/ray-rust/blob/main/docs/testing.md). The README and Markdown guides are tested by rustdoc, including setup on hidden `# ` lines.

## Changelog

See [CHANGELOG.md](https://github.com/spatie/ray-rust/blob/main/CHANGELOG.md).

## Contributing

See [CONTRIBUTING.md](https://github.com/spatie/ray-rust/blob/main/CONTRIBUTING.md) and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

Using an AI coding agent? Load Apollo's [rust-best-practices skill](https://github.com/apollographql/skills/tree/main/skills/rust-best-practices) when writing or reviewing code in this package.

## Security Vulnerabilities

Report vulnerabilities privately using the instructions in [SECURITY.md](https://github.com/spatie/ray-rust/blob/main/SECURITY.md).

## Credits

- [Freek Van der Herten](https://github.com/freekmurze)
- [spatie/ray contributors](https://github.com/spatie/ray/graphs/contributors)
- [All contributors](https://github.com/spatie/ray-rust/graphs/contributors)

## License

The MIT License (MIT). See [LICENSE.md](https://github.com/spatie/ray-rust/blob/main/LICENSE.md).
