use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrmfBlock {
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub tissue_id: u32,
    pub tick: u64,
    pub thickness: u32,
    pub anchored_ere_passes: u32,
    pub prime_factor: u32,
    pub prev_hash: String,
    pub hash: String,
    pub signature: String,
    pub retention_until: DateTime<Utc>,
    pub origin_twin: String,
    pub legal_hold: bool,
}

pub struct CrmfStorage {
    file_path: String,
}

impl CrmfStorage {
    pub fn new(path: &str) -> io::Result<Self> {
        let storage = Self {
            file_path: path.to_string(),
        };
        if !Path::new(path).exists() {
            File::create(path)?;
        }
        Ok(storage)
    }

    pub fn append_block(&self, mut block: CrmfBlock) -> io::Result<String> {
        let last_block = self.get_last_block()?;
        let prev_hash = last_block.map(|b| b.hash).unwrap_or_else(|| "0".repeat(64));

        block.prev_hash = prev_hash;
        block.hash = self.compute_hash(&block);

        // In a real implementation, we would sign here using a HSM or secure key
        block.signature = format!("SIG-{}", &block.hash[..16]);

        let mut file = OpenOptions::new().append(true).open(&self.file_path)?;

        let serialized = serde_json::to_string(&block)?;
        writeln!(file, "{}", serialized)?;

        Ok(block.hash)
    }

    pub fn get_last_block(&self) -> io::Result<Option<CrmfBlock>> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let last_line = reader.lines().last();

        match last_line {
            Some(Ok(line)) => {
                let block: CrmfBlock = serde_json::from_str(&line)?;
                Ok(Some(block))
            }
            _ => Ok(None),
        }
    }

    pub fn verify_chain(&self) -> io::Result<bool> {
        let file = File::open(&self.file_path)?;
        let reader = BufReader::new(file);
        let mut prev_hash = "0".repeat(64);

        for line in reader.lines() {
            let line = line?;
            let block: CrmfBlock = serde_json::from_str(&line)?;

            if block.prev_hash != prev_hash {
                return Ok(false);
            }

            let computed_hash = self.compute_hash(&block);
            if block.hash != computed_hash {
                return Ok(false);
            }

            prev_hash = block.hash;
        }

        Ok(true)
    }

    fn compute_hash(&self, block: &CrmfBlock) -> String {
        let mut hasher = Sha256::new();
        hasher.update(block.sequence.to_be_bytes());
        hasher.update(block.timestamp.to_rfc3339().as_bytes());
        hasher.update(block.tissue_id.to_be_bytes());
        hasher.update(block.tick.to_be_bytes());
        hasher.update(block.thickness.to_be_bytes());
        hasher.update(block.anchored_ere_passes.to_be_bytes());
        hasher.update(block.prime_factor.to_be_bytes());
        hasher.update(block.prev_hash.as_bytes());
        hasher.update(block.retention_until.to_rfc3339().as_bytes());
        hasher.update(block.origin_twin.as_bytes());
        hasher.update(if block.legal_hold { [1u8] } else { [0u8] });
        hex::encode(hasher.finalize())
    }
    /// Logs a preservation event with associated metadata.
    /// This is a lightweight append to the storage file for audit purposes.
    pub fn log_preservation_event(&self, event: &serde_json::Value) -> io::Result<()> {
        let mut file = OpenOptions::new().append(true).open(&self.file_path)?;
        let line = serde_json::to_string(event)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }
}
