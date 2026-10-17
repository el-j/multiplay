//! Decoupled port contracts for multiplay audio player.
//! Defines inbound (driving) ports for UIs/CLIs and outbound (driven) ports for hardware/providers.

#![forbid(unsafe_code)]

pub mod driven;
pub mod driving;
pub mod error;
pub mod plugin;
pub mod stream;

pub use async_trait::async_trait;
pub use driven::*;
pub use driving::*;
pub use error::*;
pub use plugin::*;
pub use stream::SharedPrecacheBuffer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ports_compile_and_traits_exist() {
        // Simple assertion to verify linkage and visibility
        let options = SearchQueryOptions {
            text: "Aphex".to_string(),
            filter_mask: amp_domain::SourceFilterMask::default(),
            timeout_millis: 3000,
        };
        assert_eq!(options.timeout_millis, 3000);
    }
}
