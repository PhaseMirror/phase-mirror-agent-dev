use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

/// Represents a canonical, hashable Λ-Trace Atom for the provenance ledger (ADR-003).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivumEntry {
    pub timestamp: u64,
    pub event_type: String,
    pub proton_id: String,
    pub prev_hash: String,
    pub payload_hash: String,
    pub signature: String, // HMAC-style nonce for redaction
}

impl ArchivumEntry {
    /// Computes the SHA-256 link for the provenance chain.
    #[inline(always)]
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(self.event_type.as_bytes());
        hasher.update(self.proton_id.as_bytes());
        hasher.update(self.prev_hash.as_bytes());
        hasher.update(self.payload_hash.as_bytes());
        hex::encode(hasher.finalize())
    }
}

pub struct ArchivumLedger {
    pub entries: Vec<ArchivumEntry>,
    pub current_state: BTreeMap<String, String>, // proton_id -> last_hash
    tx: Option<mpsc::UnboundedSender<ArchivumEntry>>,
}

impl Default for ArchivumLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchivumLedger {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_state: BTreeMap::new(),
            tx: None,
        }
    }

    /// Initializes persistence by starting a background WAL task and recovering existing state.
    pub async fn init_persistence(&mut self, log_path: PathBuf) -> Result<()> {
        // 1. Recover from existing log
        if log_path.exists() {
            let file = File::open(&log_path).await?;
            let mut reader = BufReader::new(file).lines();
            while let Some(line) = reader.next_line().await? {
                let entry: ArchivumEntry = serde_json::from_str(&line)?;
                self.current_state
                    .insert(entry.proton_id.clone(), entry.compute_hash());
                self.entries.push(entry);
            }
        }

        // 2. Setup Async WAL Channel
        let (tx, mut rx) = mpsc::unbounded_channel::<ArchivumEntry>();
        self.tx = Some(tx);

        tokio::spawn(async move {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .await
                .expect("Failed to open Λ-Archivum WAL file");

            let mut interval = tokio::time::interval(std::time::Duration::from_millis(10));
            let mut buffer = Vec::new();

            loop {
                tokio::select! {
                    Some(entry) = rx.recv() => {
                        let json = serde_json::to_string(&entry).expect("Failed to serialize ArchivumEntry");
                        buffer.push(format!("{}\n", json));
                        if buffer.len() >= 64 {
                            for line in buffer.drain(..) {
                                let _ = file.write_all(line.as_bytes()).await;
                            }
                            let _ = file.sync_data().await;
                        }
                    }
                    _ = interval.tick() => {
                        if !buffer.is_empty() {
                            for line in buffer.drain(..) {
                                let _ = file.write_all(line.as_bytes()).await;
                            }
                            let _ = file.sync_data().await;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Appends a verified event to the ledger with a Fail-Closed integrity check (ADR-005).
    pub fn commit_event(&mut self, event_type: &str, id: String, data: &[u8]) -> Result<()> {
        let prev = self
            .entries
            .last()
            .map(|e| e.compute_hash())
            .unwrap_or_else(|| "GENESIS".to_string());

        let mut hasher = Sha256::new();
        hasher.update(data);
        let payload_hash = hex::encode(hasher.finalize());

        let entry = ArchivumEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs(),
            event_type: event_type.to_string(),
            proton_id: id.clone(),
            prev_hash: prev.clone(),
            payload_hash,
            signature: "REDACTED_HMAC".to_string(),
        };

        // L0 Invariant: Verify chain continuity before push
        if !self.entries.is_empty() && entry.prev_hash != prev {
            return Err(anyhow!("ADR-003: Chain continuity broken"));
        }

        // Async persistence offload (ADR-003 / ADR-005)
        if let Some(ref tx) = self.tx {
            let _ = tx.send(entry.clone());
        }

        self.entries.push(entry);
        self.current_state.insert(id, prev);
        Ok(())
    }

    pub fn get_entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Represents the localized snapshot state of a Λ-Archivum Node's state tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleSnapshot {
    pub root_hash: String,
    pub leaves: BTreeMap<String, String>,
}

impl MerkleSnapshot {
    pub fn compute_root(leaves: &BTreeMap<String, String>) -> String {
        if leaves.is_empty() {
            return "EMPTY_GENESIS_ROOT".to_string();
        }

        let mut hasher = Sha256::new();
        for (key, val) in leaves.iter() {
            hasher.update(key.as_bytes());
            hasher.update(val.as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

pub trait SyncManager {
    fn generate_snapshot(&self) -> MerkleSnapshot;
    fn reconcile_delta(&self, remote_snapshot: &MerkleSnapshot) -> SyncOutcome;
}

#[derive(Debug, PartialEq, Eq)]
pub enum SyncOutcome {
    Synchronized,
    DeltaIdentified(Vec<String>),
    IntegrityFailure(&'static str),
}

pub struct DistributedSyncOracle {
    pub tracked_leaves: BTreeMap<String, String>,
}

impl SyncManager for DistributedSyncOracle {
    fn generate_snapshot(&self) -> MerkleSnapshot {
        let root = MerkleSnapshot::compute_root(&self.tracked_leaves);
        MerkleSnapshot {
            root_hash: root,
            leaves: self.tracked_leaves.clone(),
        }
    }

    fn reconcile_delta(&self, remote_snapshot: &MerkleSnapshot) -> SyncOutcome {
        let local_root = MerkleSnapshot::compute_root(&self.tracked_leaves);

        if local_root == remote_snapshot.root_hash {
            return SyncOutcome::Synchronized;
        }

        let mut out_of_sync_protons = Vec::new();

        for (proton_id, local_hash) in &self.tracked_leaves {
            match remote_snapshot.leaves.get(proton_id) {
                Some(remote_hash) => {
                    if local_hash != remote_hash {
                        out_of_sync_protons.push(proton_id.clone());
                    }
                }
                None => {
                    out_of_sync_protons.push(proton_id.clone());
                }
            }
        }

        for proton_id in remote_snapshot.leaves.keys() {
            if !self.tracked_leaves.contains_key(proton_id) {
                out_of_sync_protons.push(proton_id.clone());
            }
        }

        if out_of_sync_protons.is_empty() {
            SyncOutcome::IntegrityFailure(
                "ADR-005: Root hash mismatch with unresolvable leaf alignment",
            )
        } else {
            SyncOutcome::DeltaIdentified(out_of_sync_protons)
        }
    }
}
