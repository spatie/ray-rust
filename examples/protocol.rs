//! Send the PHP compatibility scenario to a capture server on the supplied port.

#[path = "../tests/common/scenario.rs"]
mod scenario;

use spatie_ray::{Client, Config, Error};

fn main() -> Result<(), Error> {
    let port = std::env::args()
        .nth(1)
        .ok_or_else(|| Error::Config("port required".into()))?
        .parse()
        .map_err(|_| Error::Config("invalid port".into()))?;
    let client = Client::new(
        Config::default()
            .host("127.0.0.1")
            .port(port)
            .throws(true)
            .project("Protocol"),
    );
    scenario::send(&client)
}
