//! BlitzHash - High-throughput non-cryptographic hash function
//! 
//! **WARNING : NOT CRYPTOGRAPHICALLY SECURE**
//! This is a performance demonstration only. Do not use for security purposes.

/// mixing constants - chosen to avoid trivial algebraic cancellation
const K1: u64 = 0x517cc1b727220a95;
const K2: u64 = 0x85ebca6b2f3c8b51;
const K3: u64 = 0xc2b2ae3d27d4eb4f;
const K4: u64 = 0x165667b19e3779f9;

/// fast mixing function for one 64-bit chunk
#[inline(always)]
fn mix_chunk(state: u64, chunk: u64, constant: u64) -> u64 {
    let mut h = state ^ chunk;
    h = h.wrapping_mul(constant);
    h ^= h.rotate_right(27);
    h = h.wrapping_mul(K1);
    h ^= h.rotate_right(31);
    h
}

/// blitzhash stateful hasher for streaming data
#[derive(Clone)]
pub struct BlitzState {
    state: [u64; 4],
    buffer: [u8; 8],
    buffer_len: usize,
    total_len: u64,
}

impl BlitzState {
    /// create a new hasher with the given seed
    pub fn new(seed: u64) -> Self {
        Self {
            state: [
                seed ^ K1,
                seed ^ K2,
                seed ^ K3,
                seed ^ K4,
            ],
            buffer: [0u8; 8],
            buffer_len: 0,
            total_len: 0,
        }
    }

    /// absorb data into the hash state
    pub fn absorb(&mut self, data: &[u8]) {
        let mut pos = 0;
        self.total_len += data.len() as u64;

        // handle buffered bytes first
        if self.buffer_len > 0 {
            let needed = 8 - self.buffer_len;
            let available = data.len().min(needed);
            self.buffer[self.buffer_len..self.buffer_len + available]
                .copy_from_slice(&data[..available]);
            self.buffer_len += available;
            pos += available;

            if self.buffer_len == 8 {
                self.process_chunk(&self.buffer);
                self.buffer_len = 0;
            }
        }

        // process full 8-byte chunks
        while pos + 8 <= data.len() {
            self.process_chunk(&data[pos..pos + 8]);
            pos += 8;
        }

        // buffer remaining bytes
        if pos < data.len() {
            let remaining = data.len() - pos;
            self.buffer[..remaining].copy_from_slice(&data[pos..]);
            self.buffer_len = remaining;
        }
    }

    #[inline(always)]
    fn process_chunk(&mut self, chunk: &[u8]) {
        let value = u64::from_le_bytes(chunk.try_into().unwrap());
        self.state[0] = mix_chunk(self.state[0], value, K1);
        self.state[1] = mix_chunk(self.state[1], value, K2);
        self.state[2] = mix_chunk(self.state[2], value, K3);
        self.state[3] = mix_chunk(self.state[3], value, K4);
    }

    /// finalize and produce the 32-byte digest
    pub fn finalize(mut self) -> [u8; 32] {
        // Process remaining buffered bytes with padding
        if self.buffer_len > 0 {
            // Pad with zeros and length marker
            for i in self.buffer_len..8 {
                self.buffer[i] = 0;
            }
            self.process_chunk(&self.buffer);
        }

        // mix in total length for length extension resistance
        self.state[0] ^= self.total_len;
        self.state[1] ^= self.total_len.rotate_right(17);
        self.state[2] ^= self.total_len.rotate_right(31);
        self.state[3] ^= self.total_len.rotate_right(47);

        // final avalanche mixing
        for _ in 0..3 {
            self.state[0] = self.state[0].wrapping_mul(K1) ^ self.state[0].rotate_right(29);
            self.state[1] = self.state[1].wrapping_mul(K2) ^ self.state[1].rotate_right(31);
            self.state[2] = self.state[2].wrapping_mul(K3) ^ self.state[2].rotate_right(33);
            self.state[3] = self.state[3].wrapping_mul(K4) ^ self.state[3].rotate_right(37);
        }

        // output 32 bytes (4 × 64-bit words)
        let mut output = [0u8; 32];
        output[0..8].copy_from_slice(&self.state[0].to_le_bytes());
        output[8..16].copy_from_slice(&self.state[1].to_le_bytes());
        output[16..24].copy_from_slice(&self.state[2].to_le_bytes());
        output[24..32].copy_from_slice(&self.state[3].to_le_bytes());
        output
    }
}

/// one-shot hashing function
pub fn blitz_hash(seed: u64, data: &[u8]) -> [u8; 32] {
    let mut hasher = BlitzState::new(seed);
    hasher.absorb(data);
    hasher.finalize()
}

/// parallel hashing using Rayon for large inputs
pub fn blitz_hash_parallel(seed: u64, data: &[u8], num_threads: usize) -> [u8; 32] {
    use rayon::prelude::*;

    if data.len() < 1_000_000 || num_threads <= 1 {
        return blitz_hash(seed, data);
    }

    let chunk_size = (data.len() + num_threads - 1) / num_threads;
    let chunks: Vec<_> = data.chunks(chunk_size).collect();

    let partial_hashes: Vec<[u8; 32]> = chunks
        .par_iter()
        .enumerate()
        .map(|(idx, chunk)| {
            blitz_hash(seed.wrapping_add(idx as u64), chunk)
        })
        .collect();

    // need to combineee partial hashes
    let mut combined = Vec::with_capacity(partial_hashes.len() * 32);
    for hash in partial_hashes {
        combined.extend_from_slice(&hash);
    }

    blitz_hash(seed, &combined)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let data = b"Hello, BlitzHash!";
        let h1 = blitz_hash(0, data);
        let h2 = blitz_hash(0, data);
        assert_eq!(h1, h2, "Same input should produce same hash");
    }

    #[test]
    fn test_different_seeds() {
        let data = b"test data";
        let h1 = blitz_hash(0, data);
        let h2 = blitz_hash(1, data);
        assert_ne!(h1, h2, "Different seeds should produce different hashes");
    }

    #[test]
    fn test_streaming_matches_oneshot() {
        let data = b"The quick brown fox jumps over the lazy dog";
        let oneshot = blitz_hash(42, data);
        
        let mut streaming = BlitzState::new(42);
        streaming.absorb(&data[..10]);
        streaming.absorb(&data[10..20]);
        streaming.absorb(&data[20..]);
        let streamed = streaming.finalize();
        
        assert_eq!(oneshot, streamed, "Streaming should match one-shot");
    }

    #[test]
    fn test_empty_input() {
        let h = blitz_hash(0, b"");
        assert_eq!(h.len(), 32);
    }

    #[test]
    fn test_single_byte() {
        let h1 = blitz_hash(0, b"a");
        let h2 = blitz_hash(0, b"b");
        assert_ne!(h1, h2);
    }
}
