---
title: Configuration
weight: 5
---

By default, Ray is listening on `localhost:23517`. You can configure another host and port using `Config`:

```rust
# fn main() -> Result<(), spatie_ray::Error> {
use spatie_ray::{Client, Config};

let client = Client::new(Config::default().host("localhost").port(23517));
client.ray().send("Hello world")?;
# Ok(())
# }
```

`Config::default()` uses these settings:

| Setting | Default |
| --- | --- |
| `host` | `localhost` |
| `port` | `23517` |
| `enabled` | `true` |
| `timeout` | Two seconds |
| `throws` | `false` |
| `project` | Empty string |
| `remote_path` | No path mapping |
| `always_send_raw_values` | `false` |

To load environment variables explicitly, use `Config::from_env()`:

```rust
# fn main() -> Result<(), spatie_ray::Error> {
# use spatie_ray::{Client, Config, ray};
let client = Client::new(Config::from_env()?);
ray!(client: client, "configured")?;
# Ok(())
# }
```

Supported variables are `RAY_HOST`, `RAY_PORT`, `RAY_ENABLED`, `RAY_PROJECT_NAME`, `RAY_REMOTE_PATH`, `RAY_LOCAL_PATH` and `RAY_ALWAYS_SEND_RAW_VALUES`. Booleans accept `true`, `false`, `1` or `0`. Invalid values return `Error::Config`.

The plain `ray!` macro uses a shared default client. Custom clients are explicit. Clones of one client share enablement, counters, timers, limits and project name; separately created clients are isolated.

## Remote debugging

Map remote source paths to your local editor paths:

```rust
# use spatie_ray::{Client, Config};
let client = Client::new(
    Config::default()
        .host("localhost")
        .remote_path("/var/www/my-project", "/Users/freek/my-project")
);
# assert!(client.ray().enabled().unwrap());
```

Only a matching prefix is replaced, in both origins and stack frames. The transport is unencrypted HTTP. Keep the port private and use an SSH tunnel for remote debugging. This client does not use HTTP proxies or follow redirects.

## Reporting transport errors

When Ray is closed or unavailable, output is skipped without failing your application. Set `throws(true)` to return transport errors instead. JSON serialization, file reading, XML parsing and shared-state errors always propagate. Queries for `windows()` and `theme()` always report failures.

```rust
# use spatie_ray::{Client, Config};
use std::time::Duration;

let client = Client::new(
    Config::default().throws(true).timeout(Duration::from_secs(2))
);
# assert!(client.ray().enabled().unwrap());
```
