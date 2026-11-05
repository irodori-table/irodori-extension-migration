//! Native tool extension: Migration Studio.
//!
//! Planning lives in the `irodori-migration` crate. This crate maps the form the
//! host renders from `irodori.extension.json` onto it, and the plan onto output
//! tabs the host knows how to show.
//!
//! A tool uses the connector ABI's symbols, so the host loads it the same way.
//! The "engine" it reports is its extension id, because it has no database.

pub use irodori_connector_abi as abi;

mod driver;
mod text;

irodori_connector_abi::irodori_export_connector!(
    engine: "irodori.migration",
    driver: driver,
    config: "../connector.config.json",
    manifest: "../irodori.extension.json",
    driver_linked: true,
);
