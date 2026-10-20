//! Cloud sync (docs/cloud-sync).
//!
//! CS-1: canonical vault export, encrypted blob codec, and import.
//! Writer, config, and UI land in CS-2/CS-3/CS-4.

pub mod blob;
pub mod vault_export;
pub mod vault_import;

#[cfg(test)]
pub(crate) mod test_util;
