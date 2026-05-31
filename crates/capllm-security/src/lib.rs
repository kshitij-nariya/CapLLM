//! Zero-trust security layer for the `CapLLM` gateway.
//!
//! This crate provides:
//! - **DLP masking**: Regex-based PII/secret detection and inline redaction
//! - **Encrypted vault**: AES-256-GCM transient storage for redacted originals
//! - **Injection guard**: Heuristic jailbreak prompt detection
//! - **ZDR enforcement**: Vendor-specific zero-data-retention headers

pub mod dlp;
pub mod injection;
pub mod vault;
pub mod zdr;

pub use dlp::DlpEngine;
pub use injection::InjectionGuard;
pub use vault::SensitiveVault;
pub use zdr::ZdrEnforcer;
