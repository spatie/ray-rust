---
title: Installation
weight: 3
---

This package can be installed in any Rust application.

Requires Rust 1.89 or newer. Install Rust using [rustup](https://rustup.rs/), and download the [Ray desktop app](https://myray.app).

Add the package to your application:

```sh
cargo add spatie-ray
```

Open Ray, then send your first message:

```rust
# fn main() -> Result<(), spatie_ray::Error> {
use spatie_ray::ray;

ray!("Hello world")?;
# Ok(())
# }
```

Ray listens on `localhost:23517` by default. See [configuration](configuration.md) for remote hosts and path mapping. No PHP installation or async runtime is needed.
