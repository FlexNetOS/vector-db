//! Model fingerprint — sha256 over HEF + tokenizer artifacts.
//!
//! ADR-167 §8.3 fleet integrity guard: coordinators refuse to mix
//! workers reporting different model fingerprints. Computed at worker
//! startup over the files actually loaded, so any swap of HEF or
//! vocab.txt produces a different fingerprint and the coordinator can
//! eject the drift.
//!
//! Format: hex-lowercase, 64 chars.

use sha2::{Digest, Sha256};
use std::path::Path;

/// Compute sha256 over (hef_bytes || vocab_bytes). Missing files are
/// treated as empty so the fingerprint is *also* a witness of which
/// files exist — a worker that loads only the HEF (no tokenizer)
/// produces a different fingerprint than a worker with both.
///
/// Returns "" when both files are missing — caller (worker startup)
/// uses empty-string as "skip the check" sentinel until step 6 lands.
pub fn compute_fingerprint(model_dir: &Path) -> String {
    let hef = std::fs::read(model_dir.join("model.hef")).unwrap_or_default();
    let vocab = std::fs::read(model_dir.join("vocab.txt")).unwrap_or_default();
    if hef.is_empty() && vocab.is_empty() {
        return String::new();
    }
    let mut h = Sha256::new();
    // Length-prefix each input so a hef of N bytes + vocab of M bytes
    // never collides with a hef of N+M bytes + empty vocab.
    h.update((hef.len() as u64).to_le_bytes());
    h.update(&hef);
    h.update((vocab.len() as u64).to_le_bytes());
    h.update(&vocab);
    let digest = h.finalize();
    hex_lower(&digest)
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(nibble(b >> 4));
        s.push(nibble(b & 0x0F));
    }
    s
}

fn nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + n - 10) as char,
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmpdir() -> tempdirlite::TempDir {
        tempdirlite::TempDir::new()
    }

    #[test]
    fn empty_dir_yields_empty_fingerprint() {
        let d = tmpdir();
        assert_eq!(compute_fingerprint(d.path()), "");
    }

    #[test]
    fn fingerprint_is_64_hex_chars() {
        let d = tmpdir();
        std::fs::File::create(d.path().join("model.hef"))
            .unwrap()
            .write_all(b"fake hef bytes")
            .unwrap();
        let fp = compute_fingerprint(d.path());
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn changing_either_file_changes_fingerprint() {
        let d = tmpdir();
        std::fs::write(d.path().join("model.hef"), b"hef-A").unwrap();
        std::fs::write(d.path().join("vocab.txt"), b"vocab-A").unwrap();
        let fp1 = compute_fingerprint(d.path());

        std::fs::write(d.path().join("vocab.txt"), b"vocab-B").unwrap();
        let fp2 = compute_fingerprint(d.path());
        assert_ne!(fp1, fp2);

        std::fs::write(d.path().join("model.hef"), b"hef-B").unwrap();
        let fp3 = compute_fingerprint(d.path());
        assert_ne!(fp2, fp3);
    }

    #[test]
    fn length_prefix_prevents_split_collision() {
        // Without length-prefixing, sha256(b"abc" || b"de") == sha256(b"ab" || b"cde").
        // With the u64 length prefix on each, they must differ.
        let d1 = tmpdir();
        std::fs::write(d1.path().join("model.hef"), b"abc").unwrap();
        std::fs::write(d1.path().join("vocab.txt"), b"de").unwrap();
        let fp1 = compute_fingerprint(d1.path());

        let d2 = tmpdir();
        std::fs::write(d2.path().join("model.hef"), b"ab").unwrap();
        std::fs::write(d2.path().join("vocab.txt"), b"cde").unwrap();
        let fp2 = compute_fingerprint(d2.path());

        assert_ne!(fp1, fp2);
    }
}

// Tiny in-tree TempDir to avoid pulling tempfile as a dev-dep just for this.
#[cfg(test)]
mod tempdirlite {
    use std::path::{Path, PathBuf};

    pub struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        pub fn new() -> Self {
            let id = std::process::id();
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let path = std::env::temp_dir().join(format!("ruvhailo-test-{}-{}", id, nanos));
            std::fs::create_dir_all(&path).unwrap();
            Self { path }
        }
        pub fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}
