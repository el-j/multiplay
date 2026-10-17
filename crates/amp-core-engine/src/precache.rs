//! In-memory stream buffer for concurrent audio precaching.
//! Re-exports `SharedPrecacheBuffer` from `amp-ports`.

#![forbid(unsafe_code)]

pub use amp_ports::SharedPrecacheBuffer;

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Read;

    proptest! {
        #[test]
        fn test_precache_arbitrary_chunk_invariants(
            chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 1..64), 1..16)
        ) {
            let total_bytes: usize = chunks.iter().map(|c| c.len()).sum();
            let mut buffer = SharedPrecacheBuffer::new(total_bytes + 1024);

            for chunk in &chunks {
                buffer.push_chunk(chunk);
            }
            buffer.mark_finished();

            prop_assert_eq!(buffer.total_bytes_cached(), total_bytes);

            let mut assembled = Vec::with_capacity(total_bytes);
            let mut read_buf = [0u8; 32];
            loop {
                let n = buffer.read(&mut read_buf).unwrap();
                if n == 0 {
                    break;
                }
                assembled.extend_from_slice(&read_buf[..n]);
            }

            let expected: Vec<u8> = chunks.into_iter().flatten().collect();
            prop_assert_eq!(assembled, expected);
        }
    }
}
