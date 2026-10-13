//! LexLean Stratum: Canonical identity, lexeme ledger, and PIRTM §34 stratification (SPEC.md §33, §34).
//!
//! Exposes exact rational contractivity checking (‖A·diag(λ)‖₁ < 1 in ℚ),
//! stratum decomposition, snapaddr generation, Zeno-Finton signaling, and
//! §33/§34 verification with strict stratification invariance.

pub mod artifact;
pub mod canonical;
pub mod child;
pub mod config;
pub mod contractivity;
pub mod diagnostic;
pub mod entry;
pub mod error;
pub mod ledger;
pub mod merkle;
pub mod pirtm;
pub mod rational;
pub mod signature;
pub mod stratum;
pub mod timestamp;
pub mod tsa_crypto;
pub mod verify;
pub mod zeno;

/// Pinned Lean toolchain
pub const LEAN_TOOLCHAIN: &str = "leanprover/lean4:v4.32.1";

/// Pinned Language version
pub const LANGUAGE_VERSION: &str = "1.0";


pub use artifact::canonical_json::Json;
pub use artifact::content_id::{FramedHasher, Sha256Digest};
pub use canonical::{canonicalize, CanonicalSource, Declaration, Token};
pub use contractivity::{
    Adjacency, Contraction, ContractivityReceipt, Scan, STATUS_ACCEPT, STATUS_REJECT,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, Span};
pub use entry::{Entry, HashedSource, Inclusion};
pub use error::{ErrorClass, LexLeanError};
pub use ledger::Ledger;
pub use pirtm::PirtmLayer;
pub use rational::Rational;
pub use stratum::{SnapAddr, Stratum};
pub use verify::{verify_entry, Check, Verdict};
pub use zeno::ZenoFinton;

/// Compatibility re-export module so `crate::lexeme::*` and `lexlean::lexeme::*` paths resolve.
pub mod lexeme {
    pub use crate::*;
}

/// The schema identifier every entry carries (SPEC.md §33.2).
pub const ENTRY_SPEC: &str = "lexlean/lexeme/1";

/// The canonicalization identifier (SPEC.md §33.1).
pub const CANONICALIZATION: &str = "lean-token-stream-v1";

/// The hash domain of the canonical form (SPEC.md §33.1).
pub const CANONICAL_DOMAIN: &str = "lexlean-lexeme-v1";

/// The signature algorithm every entry uses (SPEC.md §33.3).
pub const SIGNATURE_ALGORITHM: &str = "ed25519";

/// The default RFC 3161 time stamp authority (SPEC.md §33.4).
pub const DEFAULT_TSA_URL: &str = "https://freetsa.org/tsr";

/// The timestamp artifact selector (SPEC.md §33.6).
pub const TIMESTAMP_ARTIFACT: &str = "signature";
