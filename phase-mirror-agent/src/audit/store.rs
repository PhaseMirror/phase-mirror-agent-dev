use anyhow::{bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub const WAL_FILE: &str = "wal.jsonl";
pub const META_FILE: &str = "wal.meta.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: u64,
    pub sequence: u64,
    pub timestamp: String,
    pub event_type: String,
    pub actor: Option<String>,
    pub details: String,
    pub prev_hash: String,
    pub entry_hash: String,
}

impl AuditEntry {
    pub fn payload(&self) -> Value {
        serde_json::json!({
            "id": self.id,
            "sequence": self.sequence,
            "timestamp": self.timestamp,
            "event_type": self.event_type,
            "actor": self.actor,
            "details": self.details,
        })
    }
}

#[derive(Debug, Serialize)]
pub struct IntegrityReport {
    pub valid: bool,
    pub entries: usize,
    pub head_hash: String,
}

#[derive(Debug, Serialize)]
pub struct RotateReport {
    pub archived: PathBuf,
    pub head_hash: String,
    pub entries: usize,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WalMeta {
    /// prev_hash the first line of the current WAL must link from
    pub first_prev_hash: String,
    pub head_hash: String,
    pub next_sequence: u64,
    pub next_id: u64,
}

struct StoreInner {
    file: File,
    entries: Vec<AuditEntry>,
    next_sequence: u64,
    next_id: u64,
    head_hash: String,
    /// prev_hash the first line of the current WAL must link from
    first_prev_hash: String,
}

pub struct AuditStore {
    dir: PathBuf,
    wal_path: PathBuf,
    meta_path: PathBuf,
    inner: RwLock<StoreInner>,
}

impl AuditStore {
    pub async fn open(state_dir: &Path) -> Result<Arc<Self>> {
        let dir = state_dir.join("audit");
        fs::create_dir_all(&dir).with_context(|| format!("create audit dir {}", dir.display()))?;
        let wal_path = dir.join(WAL_FILE);
        let meta_path = dir.join(META_FILE);

        let (entries, next_sequence, next_id, head_hash, first_prev_hash) =
            Self::replay(&wal_path, &meta_path)?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&wal_path)
            .with_context(|| format!("open WAL {}", wal_path.display()))?;
        file.sync_all()?;

        let inner = StoreInner {
            file,
            entries,
            next_sequence,
            next_id,
            head_hash,
            first_prev_hash,
        };
        let store = Arc::new(Self {
            dir,
            wal_path,
            meta_path,
            inner: RwLock::new(inner),
        });
        info!(
            entries = store.inner.read().await.entries.len(),
            head_hash = %store.inner.read().await.head_hash,
            "audit store opened"
        );
        Ok(store)
    }

    fn replay(
        wal_path: &Path,
        meta_path: &Path,
    ) -> Result<(Vec<AuditEntry>, u64, u64, String, String)> {
        let meta = if meta_path.exists() {
            Some(serde_json::from_str::<WalMeta>(&fs::read_to_string(
                meta_path,
            )?)?)
        } else {
            None
        };

        let mut entries = Vec::new();
        let mut head_hash = GENESIS_HASH.to_string();
        let mut next_sequence = 0u64;
        let mut next_id = 0u64;
        let first_prev_hash = meta
            .as_ref()
            .map(|m| m.first_prev_hash.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string());

        if wal_path.exists() {
            let file = File::open(wal_path)
                .with_context(|| format!("open WAL for replay {}", wal_path.display()))?;
            let reader = BufReader::new(file);
            let mut prev_hash = first_prev_hash.clone();
            for (lineno, line) in reader.lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let entry: AuditEntry = serde_json::from_str(&line)
                    .with_context(|| format!("corrupt WAL line {}", lineno + 1))?;
                if entry.prev_hash != prev_hash {
                    bail!(
                        "WAL chain broken at line {}: prev_hash {} != expected {}",
                        lineno + 1,
                        entry.prev_hash,
                        prev_hash
                    );
                }
                let expected = Self::entry_hash(&entry.prev_hash, &entry.payload())?;
                if expected != entry.entry_hash {
                    bail!(
                        "WAL chain hash mismatch at line {}: entry_hash {} != expected {}",
                        lineno + 1,
                        entry.entry_hash,
                        expected
                    );
                }
                prev_hash = entry.entry_hash.clone();
                next_sequence = entry.sequence + 1;
                next_id = entry.id + 1;
                entries.push(entry);
            }
            if let Some(last) = entries.last() {
                head_hash = last.entry_hash.clone();
            }
        }

        if entries.is_empty() {
            if let Some(meta) = meta {
                head_hash = meta.head_hash;
                next_sequence = meta.next_sequence;
                next_id = meta.next_id;
            }
        }

        Ok((entries, next_sequence, next_id, head_hash, first_prev_hash))
    }

    pub fn entry_hash(prev_hash: &str, payload: &Value) -> Result<String> {
        let canonical = canonical_json(payload)?;
        let mut hasher = Sha256::new();
        hasher.update(prev_hash.as_bytes());
        hasher.update(canonical.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    pub async fn append(
        self: &Arc<Self>,
        event_type: String,
        actor: Option<String>,
        details: String,
    ) -> Result<AuditEntry> {
        let mut inner = self.inner.write().await;
        let timestamp = Utc::now().to_rfc3339();
        let prev_hash = if inner.entries.is_empty() {
            inner.first_prev_hash.clone()
        } else {
            inner.head_hash.clone()
        };
        let entry = AuditEntry {
            id: inner.next_id,
            sequence: inner.next_sequence,
            timestamp,
            event_type,
            actor,
            details,
            prev_hash: prev_hash.clone(),
            entry_hash: String::new(),
        };
        let payload = entry.payload();
        let entry_hash = Self::entry_hash(&prev_hash, &payload)?;
        let mut entry = entry;
        entry.entry_hash = entry_hash;

        let line = serde_json::to_string(&entry)?;
        writeln!(inner.file, "{line}")?;
        inner.file.sync_all()?;

        inner.entries.push(entry.clone());
        inner.next_sequence += 1;
        inner.next_id += 1;
        inner.head_hash = entry.entry_hash.clone();
        info!(id = %entry.id, sequence = %entry.sequence, "audit entry recorded");
        Ok(entry)
    }

    pub async fn list(self: &Arc<Self>) -> Vec<AuditEntry> {
        self.inner.read().await.entries.clone()
    }

    pub async fn get(self: &Arc<Self>, id: u64) -> Option<AuditEntry> {
        self.inner
            .read()
            .await
            .entries
            .iter()
            .find(|e| e.id == id)
            .cloned()
    }

    pub async fn summary(self: &Arc<Self>) -> (usize, std::collections::HashMap<String, usize>) {
        let entries = self.inner.read().await.entries.clone();
        let mut by_type = std::collections::HashMap::new();
        for e in &entries {
            *by_type.entry(e.event_type.clone()).or_default() += 1;
        }
        (entries.len(), by_type)
    }

    /// Flush the WAL file and persist the chain metadata (ADR-007 §2.4 step 4).
    /// Every `append` already fsyncs the file; this additionally persists
    /// `wal.meta.json` and the directory entry so a shutdown is durable even
    /// when no new entry was written.
    pub async fn flush(self: &Arc<Self>) -> Result<()> {
        let inner = self.inner.write().await;
        inner.file.sync_all()?;
        let meta = WalMeta {
            first_prev_hash: inner.first_prev_hash.clone(),
            head_hash: inner.head_hash.clone(),
            next_sequence: inner.next_sequence,
            next_id: inner.next_id,
        };
        Self::write_meta(&self.meta_path, &meta)?;
        Self::fsync_dir(&self.dir)?;
        info!(entries = %inner.entries.len(), "audit WAL flushed on shutdown");
        Ok(())
    }

    /// Readiness probe (ADR-007 §2.3): the WAL must open for append and the
    /// chain head must parse/verify. Returns an error when either fails so
    /// `/ready` can answer 503.
    pub async fn check_ready(self: &Arc<Self>) -> Result<()> {
        let inner = self.inner.read().await;
        let report = Self::verify_file(&self.wal_path, &inner.first_prev_hash)?;
        if !report.valid {
            bail!("audit WAL chain head failed to parse");
        }
        let file = OpenOptions::new()
            .append(true)
            .open(&self.wal_path)
            .with_context(|| format!("audit WAL not writable: {}", self.wal_path.display()))?;
        file.sync_all()?;
        Ok(())
    }

    pub async fn verify_integrity(self: &Arc<Self>) -> Result<IntegrityReport> {
        let inner = self.inner.read().await;
        let report = Self::verify_file(&self.wal_path, &inner.first_prev_hash)?;
        Ok(report)
    }

    fn verify_file(wal_path: &Path, first_prev_hash: &str) -> Result<IntegrityReport> {
        let mut prev_hash = first_prev_hash.to_string();
        let mut count = 0usize;
        let mut head = first_prev_hash.to_string();
        if !wal_path.exists() {
            return Ok(IntegrityReport {
                valid: true,
                entries: 0,
                head_hash: head,
            });
        }
        let file = File::open(wal_path)?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let entry: AuditEntry = serde_json::from_str(&line)?;
            if entry.prev_hash != prev_hash {
                return Ok(IntegrityReport {
                    valid: false,
                    entries: count,
                    head_hash: head,
                });
            }
            let expected = Self::entry_hash(&entry.prev_hash, &entry.payload())?;
            if expected != entry.entry_hash {
                return Ok(IntegrityReport {
                    valid: false,
                    entries: count,
                    head_hash: head,
                });
            }
            prev_hash = entry.entry_hash.clone();
            head = entry.entry_hash.clone();
            count += 1;
        }
        Ok(IntegrityReport {
            valid: true,
            entries: count,
            head_hash: head,
        })
    }

    pub async fn rotate(self: &Arc<Self>) -> Result<RotateReport> {
        let mut inner = self.inner.write().await;
        inner.file.sync_all()?;

        let stamp = Utc::now().format("%Y%m%dT%H%M%S");
        let archived = self.dir.join(format!("wal-{stamp}.jsonl"));
        fs::rename(&self.wal_path, &archived)
            .with_context(|| format!("archive WAL to {}", archived.display()))?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&self.wal_path)?;
        file.sync_all()?;
        inner.file = file;

        let head_hash = inner.head_hash.clone();
        inner.first_prev_hash = head_hash.clone();
        let meta = WalMeta {
            first_prev_hash: head_hash.clone(),
            head_hash: head_hash.clone(),
            next_sequence: inner.next_sequence,
            next_id: inner.next_id,
        };
        Self::write_meta(&self.meta_path, &meta)?;
        Self::fsync_dir(&self.dir)?;

        info!(
            archived = %archived.display(),
            entries = %inner.entries.len(),
            "audit WAL rotated"
        );
        Ok(RotateReport {
            archived,
            head_hash,
            entries: inner.entries.len(),
        })
    }

    fn write_meta(meta_path: &Path, meta: &WalMeta) -> Result<()> {
        let tmp = meta_path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string(meta)?)?;
        File::open(&tmp)?.sync_all()?;
        fs::rename(&tmp, meta_path)?;
        Ok(())
    }

    fn fsync_dir(dir: &Path) -> Result<()> {
        #[cfg(unix)]
        {
            File::open(dir)?.sync_all()?;
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
        }
        Ok(())
    }
}

pub fn canonical_json(value: &Value) -> Result<String> {
    fn write(v: &Value) -> Result<String> {
        match v {
            Value::Null => Ok("null".to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Number(n) => Ok(canonical_number(n)),
            Value::String(s) => Ok(serde_json::to_string(s)?),
            Value::Array(items) => {
                let mut parts = Vec::with_capacity(items.len());
                for item in items {
                    parts.push(write(item)?);
                }
                Ok(format!("[{}]", parts.join(",")))
            }
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                let mut parts = Vec::with_capacity(keys.len());
                for key in keys {
                    parts.push(format!(
                        "{}:{}",
                        serde_json::to_string(key)?,
                        write(&map[key])?
                    ));
                }
                Ok(format!("{{{}}}", parts.join(",")))
            }
        }
    }
    write(value)
}

fn canonical_number(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(f) = n.as_f64() {
        return canonical_float(f);
    }
    n.to_string()
}

fn canonical_float(f: f64) -> String {
    if f == 0.0 {
        return "0".to_string();
    }
    if f.fract() == 0.0 && f.abs() < 1e15 {
        return format!("{}", f as i64);
    }
    format!("{f}")
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn temp_state_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("phase-mirror-agent-test")
            .join(name)
            .join(format!(
                "{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn appends_chain_and_survives_reopen() {
        let dir = temp_state_dir("reopen").await;
        let store = AuditStore::open(&dir).await.unwrap();
        let a = store
            .append("command".into(), Some("op".into()), "deploy x".into())
            .await
            .unwrap();
        let b = store
            .append("veto".into(), None, "rejected".into())
            .await
            .unwrap();
        assert_eq!(a.sequence, 0);
        assert_eq!(b.sequence, 1);
        assert_eq!(a.prev_hash, GENESIS_HASH);
        assert_eq!(b.prev_hash, a.entry_hash);
        assert_ne!(a.entry_hash, b.entry_hash);

        drop(store);
        let reopened = AuditStore::open(&dir).await.unwrap();
        let entries = reopened.list().await;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence, 0);
        assert_eq!(entries[1].sequence, 1);
        let c = reopened
            .append("command".into(), None, "deploy y".into())
            .await
            .unwrap();
        assert_eq!(c.sequence, 2, "sequence must continue after reopen");
        assert_eq!(
            c.prev_hash, b.entry_hash,
            "chain must continue after reopen"
        );
    }

    #[tokio::test]
    async fn integrity_valid_then_false_after_tamper() {
        let dir = temp_state_dir("tamper").await;
        let store = AuditStore::open(&dir).await.unwrap();
        for i in 0..3 {
            store
                .append("command".into(), None, format!("deploy {i}"))
                .await
                .unwrap();
        }
        let report = store.verify_integrity().await.unwrap();
        assert!(report.valid);
        assert_eq!(report.entries, 3);

        drop(store);
        let wal = dir.join("audit").join(WAL_FILE);
        let mut content = fs::read_to_string(&wal).unwrap();
        content = content.replacen("deploy 1", "deploy 9", 1);
        fs::write(&wal, content).unwrap();

        let store = AuditStore::open(&dir).await;
        assert!(store.is_err(), "tampered WAL must fail replay on open");
    }

    #[tokio::test]
    async fn rotation_preserves_chain_continuity() {
        let dir = temp_state_dir("rotate").await;
        let store = AuditStore::open(&dir).await.unwrap();
        store
            .append("command".into(), None, "one".into())
            .await
            .unwrap();
        let b = store
            .append("command".into(), None, "two".into())
            .await
            .unwrap();
        let report = store.rotate().await.unwrap();
        assert_eq!(report.head_hash, b.entry_hash);
        assert!(report.archived.exists());

        let c = store
            .append("veto".into(), None, "three".into())
            .await
            .unwrap();
        assert_eq!(c.sequence, 2, "sequence continues past archived WAL");
        assert_eq!(c.prev_hash, b.entry_hash, "post-rotation chain continuity");

        drop(store);
        let reopened = AuditStore::open(&dir).await.unwrap();
        let entries = reopened.list().await;
        assert_eq!(
            entries.len(),
            1,
            "live WAL holds only post-rotation entries"
        );
        assert_eq!(
            entries[0].prev_hash, b.entry_hash,
            "rotated head survives reopen"
        );
        let integrity = reopened.verify_integrity().await.unwrap();
        assert!(integrity.valid);
        assert_eq!(integrity.head_hash, c.entry_hash);
    }

    #[tokio::test]
    async fn integrity_flips_false_when_file_tampered_while_running() {
        let dir = temp_state_dir("tamper_runtime").await;
        let store = AuditStore::open(&dir).await.unwrap();
        for i in 0..3 {
            store
                .append("command".into(), None, format!("deploy {i}"))
                .await
                .unwrap();
        }
        assert!(store.verify_integrity().await.unwrap().valid);

        let wal = dir.join("audit").join(WAL_FILE);
        let mut content = fs::read_to_string(&wal).unwrap();
        content = content.replacen("deploy 1", "deploy 9", 1);
        fs::write(&wal, content).unwrap();

        let report = store.verify_integrity().await.unwrap();
        assert!(!report.valid, "on-disk tampering must be detected");
    }

    #[test]
    fn canonical_json_sorts_keys_and_handles_numbers() {
        let v = serde_json::json!({
            "b": 2,
            "a": [true, null, {"y": 1.5, "x": "s"}],
        });
        assert_eq!(
            canonical_json(&v).unwrap(),
            r#"{"a":[true,null,{"x":"s","y":1.5}],"b":2}"#
        );
    }

    #[test]
    fn canonical_float_formats_like_shortest_roundtrip() {
        assert_eq!(canonical_float(0.84), "0.84");
        assert_eq!(canonical_float(209.3), "209.3");
        assert_eq!(canonical_float(2.0), "2");
        assert_eq!(canonical_float(0.0), "0");
    }

    #[test]
    fn entry_hash_is_deterministic() {
        let payload = serde_json::json!({
            "id": 1, "sequence": 1, "timestamp": "t",
            "event_type": "cmd", "actor": null, "details": "d"
        });
        let h1 = AuditStore::entry_hash(GENESIS_HASH, &payload).unwrap();
        let h2 = AuditStore::entry_hash(GENESIS_HASH, &payload).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn ts_witness_fixture_verifies_with_shared_algorithm() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("ts-witnesses.jsonl");
        let content = fs::read_to_string(&fixture)
            .unwrap_or_else(|e| panic!("missing TS fixture {}: {e}", fixture.display()));
        let mut prev = GENESIS_HASH.to_string();
        let mut count = 0usize;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(line).expect("fixture line must parse");
            let rec = value.as_object().expect("fixture entry must be an object");
            let entry_hash = rec.get("entry_hash").and_then(Value::as_str).unwrap();
            let prev_hash = rec.get("prev_hash").and_then(Value::as_str).unwrap();
            assert_eq!(
                prev_hash, prev,
                "fixture chain link broken at entry {count}"
            );

            let mut payload = rec.clone();
            payload.remove("prev_hash");
            payload.remove("entry_hash");
            let expected = AuditStore::entry_hash(prev_hash, &Value::Object(payload)).unwrap();
            assert_eq!(
                expected, entry_hash,
                "TS/Rust hash mismatch at entry {count}"
            );
            prev = entry_hash.to_string();
            count += 1;
        }
        assert_eq!(count, 3, "fixture must contain exactly 3 entries");
        assert_eq!(prev.len(), 64);
    }
}
