---
title: Introduction
weight: 1
---

This package can be installed in any Rust application to send debug output to [Ray](https://myray.app), the desktop debugging app from [Spatie](https://spatie.be/).

```rust
# fn main() -> Result<(), spatie_ray::Error> {
use spatie_ray::ray;

ray!("Hello world")?.green()?;
# Ok(())
# }
```

Continue with [installation](installation-and-setup.md) and [usage](usage.md).

Send structured values with Serde, or use `debug` for `Debug`-only types. Configure explicit clients when you need isolated sessions.
