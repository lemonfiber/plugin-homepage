//! What this adapter says it is, and the kit that serves its contract.

use std::sync::Arc;

use lemonfiber_adapter_kit::{Kit, Readiness};
use lemonfiber_contract::adapter::{About, Release};
use lemonfiber_contract::capabilities::dashboard;

use crate::homepage::Homepage;

/// Where Homepage's configuration directory is, in the adapter's container, which the
/// adapter shares.
pub const ADDRESS: &str = "/homepage";

/// The upstream this adapter fronts, as its project names itself.
pub const UPSTREAM: &str = "Homepage";

/// The Homepage release this adapter's recordings were taken from.
pub const RELEASE: &str = "v1.13.2";

/// The digest of the image that release's recordings were taken from.
pub const DIGEST: &str = "sha256:a0b71c8e757298d02560186bab9fbe3fc2d375c523a62cc1019177b37e48aa28";

/// What this adapter says it is: the contract it speaks and the release it supports.
#[must_use]
pub fn about() -> About {
    About {
        speaks: vec![lemonfiber_contract::spoken(
            dashboard::show::CAPABILITY,
            dashboard::show::MAJOR,
        )],
        upstream: UPSTREAM.to_owned(),
        releases: vec![Release {
            version: RELEASE.to_owned(),
            digest: DIGEST.to_owned(),
        }],
    }
}

/// The contract, served from `homepage` under `key`, ready while Homepage's configuration
/// directory is there to write.
#[must_use]
pub fn kit(key: String, homepage: Arc<Homepage>) -> Kit {
    let asked = Arc::clone(&homepage);
    let ready: Readiness = Arc::new(move || {
        let asked = Arc::clone(&asked);
        Box::pin(async move { asked.answers().await })
    });
    Kit::new(key, about())
        .ready_when(ready)
        .serving(dashboard::show::served(homepage))
}

#[cfg(test)]
mod tests;
