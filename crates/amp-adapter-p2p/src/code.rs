//! Mesh codes: the 32-byte pre-shared key of a private mesh, written as Crockford base32 in
//! groups of four (`7KQ2-M9XA-…`). Only devices with the code can connect at all (libp2p pnet).

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use std::fmt;
use std::str::FromStr;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const CHARS: usize = 52; // 256 bits in 5-bit characters

/// A private mesh's key.
#[derive(Clone, PartialEq, Eq)]
pub struct MeshCode(pub(crate) [u8; 32]);

impl fmt::Debug for MeshCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MeshCode({}…)", &self.discovery_id()[..4])
    }
}

impl MeshCode {
    /// A new random code.
    #[must_use]
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
        Self(key)
    }

    /// Identifies the mesh on the local network (mDNS) without revealing the key: the first 8
    /// bytes of SHA-256 over a label and the key, in hex.
    #[must_use]
    pub fn discovery_id(&self) -> String {
        let mut h = Sha256::new();
        h.update(b"multiplay-mesh-discovery-v1");
        h.update(self.0);
        h.finalize()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

impl fmt::Display for MeshCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bits: u32 = 0;
        let mut nbits = 0;
        let mut out = String::new();
        for &b in &self.0 {
            bits = (bits << 8) | u32::from(b);
            nbits += 8;
            while nbits >= 5 {
                nbits -= 5;
                out.push(ALPHABET[((bits >> nbits) & 31) as usize] as char);
            }
        }
        if nbits > 0 {
            out.push(ALPHABET[((bits << (5 - nbits)) & 31) as usize] as char);
        }
        let grouped: Vec<String> = out
            .as_bytes()
            .chunks(4)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect();
        f.write_str(&grouped.join("-"))
    }
}

/// Why a typed code was not accepted.
#[derive(Debug, thiserror::Error)]
#[error("that is not a mesh code (52 letters and digits)")]
pub struct BadMeshCode;

impl FromStr for MeshCode {
    type Err = BadMeshCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let chars: Vec<u8> = s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .map(|c| match c.to_ascii_uppercase() {
                'I' | 'L' => b'1',
                'O' => b'0',
                c => c as u8,
            })
            .collect();
        if chars.len() != CHARS {
            return Err(BadMeshCode);
        }
        let mut key = Vec::with_capacity(32);
        let (mut bits, mut nbits) = (0u32, 0);
        for c in chars {
            let v = ALPHABET.iter().position(|a| *a == c).ok_or(BadMeshCode)? as u32;
            bits = (bits << 5) | v;
            nbits += 5;
            if nbits >= 8 {
                nbits -= 8;
                key.push((bits >> nbits) as u8);
                bits &= (1 << nbits) - 1;
            }
        }
        if bits != 0 {
            return Err(BadMeshCode);
        }
        key.try_into().map(Self).map_err(|_| BadMeshCode)
    }
}
