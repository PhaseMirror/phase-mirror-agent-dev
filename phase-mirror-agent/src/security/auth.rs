use axum::{
    extract::{ConnectInfo, FromRequestParts, Request, State},
    http::{header, request::Parts, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

pub const KEY_PREFIX: &str = "pmr_op_";
pub const READ_SCOPE: &str = "operator:read";
pub const WRITE_SCOPE: &str = "operator:write";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Read,
    Write,
}

impl Scope {
    pub fn parse(s: &str) -> Option<Scope> {
        match s {
            READ_SCOPE => Some(Scope::Read),
            WRITE_SCOPE => Some(Scope::Write),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Scope::Read => READ_SCOPE,
            Scope::Write => WRITE_SCOPE,
        }
    }

    pub fn permits(&self, required: Scope) -> bool {
        match required {
            Scope::Read => true,
            Scope::Write => *self == Scope::Write,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoredKey {
    pub id: String,
    pub scope: Scope,
    pub digest: String,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub actor: String,
    pub scope: Scope,
}

pub struct Keyring {
    keys: Vec<StoredKey>,
}

impl Keyring {
    pub fn new(keys: Vec<StoredKey>) -> Self {
        Self { keys }
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn ids(&self) -> Vec<&str> {
        self.keys.iter().map(|k| k.id.as_str()).collect()
    }

    /// Parse a whitespace/comma separated list of `id:scope:secret` entries.
    /// The secret may be a raw `pmr_op_<32-hex>` key (hashed at load) or an
    /// already-hashed SHA-256 hex digest. Only digests are retained in memory.
    pub fn parse_entries(input: &str) -> Result<Vec<StoredKey>, String> {
        let mut keys = Vec::new();
        for line in input.split([',', '\n', '\r']) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() != 4 {
                return Err(format!(
                    "invalid operator key entry (expected id:scope:secret): '{line}'"
                ));
            }
            let id = parts[0].trim();
            let scope = Scope::parse(&format!("{}:{}", parts[1].trim(), parts[2].trim()))
                .ok_or_else(|| {
                    format!(
                        "invalid scope '{}.{}' in key entry (expected {READ_SCOPE} or {WRITE_SCOPE})",
                        parts[1].trim(),
                        parts[2].trim()
                    )
                })?;
            if id.is_empty() {
                return Err("operator key entry has an empty id".to_string());
            }
            let secret = parts[3].trim();
            let digest = if matches_key_format(secret) {
                sha256_hex(secret)
            } else if secret.len() == 64 && secret.chars().all(|c| c.is_ascii_hexdigit()) {
                secret.to_ascii_lowercase()
            } else {
                return Err(format!(
                    "operator key secret for '{id}' is neither a {KEY_PREFIX} key nor a SHA-256 hex digest"
                ));
            };
            keys.push(StoredKey {
                id: id.to_string(),
                scope,
                digest,
            });
        }
        Ok(keys)
    }

    /// Load the keyring from an inline entry string and/or a `0600` secrets file.
    /// A missing file is only an error when `required` is set (production mode).
    pub fn load(inline: &str, file: Option<&Path>, required: bool) -> Result<Keyring, String> {
        let mut entries = inline.to_string();
        if let Some(path) = file {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    entries.push('\n');
                    entries.push_str(&content);
                }
                Err(e) if required => {
                    return Err(format!(
                        "operator keys file {} is required in production mode: {}",
                        path.display(),
                        e
                    ));
                }
                Err(_) => {}
            }
        }
        let keys = Self::parse_entries(&entries)?;
        if required && keys.is_empty() {
            return Err(
                "no operator keys provisioned (PHASE_MIRROR_OPERATOR_KEYS or the keys file is empty); refusing to start in production mode"
                    .to_string(),
            );
        }
        Ok(Keyring::new(keys))
    }

    /// Constant-time lookup: hash the presented key and compare digests.
    pub fn verify(&self, presented: &str) -> Option<&StoredKey> {
        let digest = sha256_hex(presented);
        let presented_bytes = digest.as_bytes();
        self.keys
            .iter()
            .find(|k| ct_eq(presented_bytes, k.digest.as_bytes()))
    }
}

pub fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

/// Valid key shape: `pmr_op_` followed by 32 lowercase hex chars.
pub fn matches_key_format(key: &str) -> bool {
    match key.strip_prefix(KEY_PREFIX) {
        Some(rest) => rest.len() == 32 && rest.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    }
}

/// Length-aware constant-time equality. Returns false early on length mismatch
/// but never short-circuits on byte content.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

pub fn strip_bearer(header: &HeaderValue) -> Option<&str> {
    let value = header.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
}

/// Extracts the `AuthContext` inserted by the `authenticate` middleware.
/// Absent when a handler is invoked directly in unit tests (no middleware).
#[derive(Debug, Clone, Default)]
pub struct Authenticated(pub Option<AuthContext>);

#[axum::async_trait]
impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(Authenticated(
            parts.extensions.get::<AuthContext>().cloned(),
        ))
    }
}

/// Middleware state shared by the authn layer.
pub struct AuthState {
    pub keyring: Arc<Keyring>,
    /// Loopback exemption (read methods only, dev convenience).
    pub loopback: bool,
}

/// Extract the loopback status of the request's peer when served with
/// `into_make_service_with_connect_info::<SocketAddr>()`.
fn is_loopback(req: &Request) -> bool {
    req.extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|ci| ci.0.ip().is_loopback())
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Bearer")],
    )
        .into_response()
}

fn forbidden() -> Response {
    StatusCode::FORBIDDEN.into_response()
}

/// Authentication middleware (ADR-006 §2.1). Runs before rate limiting so a
/// valid actor identity is available to key per-client buckets.
pub async fn authenticate(
    State(state): State<Arc<AuthState>>,
    mut req: Request,
    next: Next,
) -> Response {
    let is_read = matches!(
        req.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    );

    if state.loopback && is_read && is_loopback(&req) {
        req.extensions_mut().insert(AuthContext {
            actor: "loopback".to_string(),
            scope: Scope::Read,
        });
        return next.run(req).await;
    }

    let raw_header = req.headers().get(header::AUTHORIZATION).cloned();
    let presented = raw_header.as_ref().and_then(strip_bearer);

    match presented {
        Some(key) if matches_key_format(key) => match state.keyring.verify(key) {
            Some(stored) => {
                let actor = stored.id.clone();
                let scope = stored.scope;
                req.extensions_mut().insert(AuthContext {
                    actor: actor.clone(),
                    scope,
                });
                tracing::debug!(%actor, scope = %scope.as_str(), "request authenticated");
                next.run(req).await
            }
            None => {
                tracing::debug!("authentication rejected: unknown key");
                unauthorized()
            }
        },
        _ => {
            tracing::debug!("authentication rejected: missing or malformed bearer token");
            unauthorized()
        }
    }
}

/// Authorization middleware (ADR-006 §2.2). Write methods require
/// `operator:write`; read methods are allowed for any authenticated scope.
/// Runs innermost, after `authenticate` has populated the extension.
pub async fn require_write(req: Request, next: Next) -> Response {
    let is_read = matches!(
        req.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    );
    if is_read {
        return next.run(req).await;
    }
    match req.extensions().get::<AuthContext>() {
        Some(auth) if auth.scope.permits(Scope::Write) => next.run(req).await,
        _ => forbidden(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRITE_KEY: &str = "pmr_op_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const READ_KEY: &str = "pmr_op_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const OTHER_KEY: &str = "pmr_op_cccccccccccccccccccccccccccccccc";

    fn keyring() -> Keyring {
        let entries = format!(
            "alice:operator:write:{}\ncarol:operator:read:{}",
            sha256_hex(WRITE_KEY),
            sha256_hex(READ_KEY)
        );
        Keyring::new(Keyring::parse_entries(&entries).unwrap())
    }

    #[test]
    fn key_format_is_validated() {
        assert!(matches_key_format(WRITE_KEY));
        assert!(!matches_key_format("not-a-key"));
        assert!(!matches_key_format("pmr_op_XYZ"));
        assert!(!matches_key_format(&format!("pmr_op_{}", "f".repeat(31))));
    }

    #[test]
    fn hashed_at_rest_roundtrip_verifies() {
        let ring = keyring();
        let stored = ring.verify(WRITE_KEY).expect("write key verified");
        assert_eq!(stored.id, "alice");
        assert_eq!(stored.scope, Scope::Write);
        assert!(!stored.digest.contains(WRITE_KEY), "raw key never retained");
        assert_eq!(
            stored.digest,
            sha256_hex(WRITE_KEY),
            "digest is sha256 of the key"
        );
        assert!(ring.verify(OTHER_KEY).is_none(), "unknown key rejected");
    }

    #[test]
    fn raw_key_entries_are_hashed_at_load() {
        let entries = format!("bob:operator:write:{READ_KEY}");
        let keys = Keyring::parse_entries(&entries).unwrap();
        assert_eq!(keys[0].digest, sha256_hex(READ_KEY));
        assert!(!keys[0].digest.contains(READ_KEY));
    }

    #[test]
    fn scopes_parse_and_permit() {
        assert_eq!(Scope::parse("operator:read"), Some(Scope::Read));
        assert_eq!(Scope::parse("operator:write"), Some(Scope::Write));
        assert_eq!(Scope::parse("admin"), None);
        assert!(Scope::Read.permits(Scope::Read));
        assert!(!Scope::Read.permits(Scope::Write));
        assert!(Scope::Write.permits(Scope::Read));
        assert!(Scope::Write.permits(Scope::Write));
    }

    #[test]
    fn malformed_entries_are_rejected() {
        assert!(Keyring::parse_entries("alice:operator:write").is_err());
        assert!(Keyring::parse_entries("alice:admin:notvalid").is_err());
        assert!(Keyring::parse_entries(":operator:write:xxxx").is_err());
        assert!(Keyring::parse_entries("alice:operator:write:zz").is_err());
    }

    #[test]
    fn file_loading_merges_inline_and_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("operator.keys");
        std::fs::write(
            &file,
            format!("alice:operator:write:{}", sha256_hex(WRITE_KEY)),
        )
        .unwrap();
        let ring = Keyring::load("", Some(&file), true).unwrap();
        assert!(ring.verify(WRITE_KEY).is_some());
        assert!(ring.verify(READ_KEY).is_none());
    }

    #[test]
    fn missing_required_file_is_error() {
        assert!(Keyring::load("", Some(Path::new("/nonexistent/keys")), true).is_err());
        assert!(Keyring::load("", Some(Path::new("/nonexistent/keys")), false).is_ok());
    }
}
