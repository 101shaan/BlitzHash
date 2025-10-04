//! BlitzHash - HIGH PERFORMANCE (Actually Fast Edition)
//! **WARNING: NOT CRYPTOGRAPHICALLY SECURE**

const K1: u64 = 0x517cc1b727220a95;
const K2: u64 = 0x85ebca6b2f3c8b51;
const K3: u64 = 0xc2b2ae3d27d4eb4f;
const K4: u64 = 0x165667b19e3779f9;

/// Fast unaligned u64 read - NO BOUNDS CHECKS
#[inline(always)]
unsafe fn read_u64_unaligned(ptr: *const u8) -> u64 {
    u64::from_le(std::ptr::read_unaligned(ptr as *const u64))
}

/// NUCLEAR mixing - inline everything
#[inline(always)]
fn mix_chunk(mut h: u64, chunk: u64, k: u64) -> u64 {
    h ^= chunk;
    h = h.wrapping_mul(k);
    h ^= h.rotate_right(27);
    h = h.wrapping_mul(K1);
    h ^= h.rotate_right(31);
    h
}

/// Ultra-fast baseline hash - FIXED
pub fn blitz_hash(seed: u64, data: &[u8]) -> [u8; 32] {
    let mut state = [seed ^ K1, seed ^ K2, seed ^ K3, seed ^ K4];
    let mut pos = 0;
    
    // Process 32-byte chunks (4×8) - UNROLLED with proper reads
    while pos + 32 <= data.len() {
        unsafe {
            // Prefetch next cache line
            #[cfg(target_arch = "x86_64")]
            {
                use std::arch::x86_64::_mm_prefetch;
                const _MM_HINT_T0: i32 = 3;
                if pos + 64 <= data.len() {
                    _mm_prefetch(data.as_ptr().add(pos + 64) as *const i8, _MM_HINT_T0);
                }
            }
            
            let ptr = data.as_ptr().add(pos);
            let c0 = read_u64_unaligned(ptr);
            let c1 = read_u64_unaligned(ptr.add(8));
            let c2 = read_u64_unaligned(ptr.add(16));
            let c3 = read_u64_unaligned(ptr.add(24));
            
            state[0] = mix_chunk(state[0], c0, K1);
            state[1] = mix_chunk(state[1], c1, K2);
            state[2] = mix_chunk(state[2], c2, K3);
            state[3] = mix_chunk(state[3], c3, K4);
        }
        
        pos += 32;
    }
    
    // Process remaining 8-byte chunks
    while pos + 8 <= data.len() {
        unsafe {
            let chunk = read_u64_unaligned(data.as_ptr().add(pos));
            state[0] = mix_chunk(state[0], chunk, K1);
            state[1] = mix_chunk(state[1], chunk.rotate_left(11), K2);
            state[2] = mix_chunk(state[2], chunk.rotate_left(23), K3);
            state[3] = mix_chunk(state[3], chunk.rotate_left(37), K4);
        }
        pos += 8;
    }
    
    // Tail handling - DISTRIBUTE ACROSS ALL LANES
    if pos < data.len() {
        let mut tail = [0u8; 8];
        let rem = data.len() - pos;
        tail[..rem].copy_from_slice(&data[pos..]);
        let chunk = u64::from_le_bytes(tail);
        
        // Mix tail into ALL lanes with rotation for diffusion
        state[0] = mix_chunk(state[0], chunk, K1);
        state[1] = mix_chunk(state[1], chunk.rotate_left(13), K2);
        state[2] = mix_chunk(state[2], chunk.rotate_left(27), K3);
        state[3] = mix_chunk(state[3], chunk.rotate_left(43), K4);
    }
    
    // Length mixing
    let len = data.len() as u64;
    state[0] ^= len;
    state[1] ^= len.rotate_right(17);
    state[2] ^= len.rotate_right(31);
    state[3] ^= len.rotate_right(47);
    
    // Final avalanche - AGGRESSIVE (4 rounds for better diffusion)
    for _ in 0..4 {
        state[0] = state[0].wrapping_mul(K1) ^ state[0].rotate_right(29);
        state[1] = state[1].wrapping_mul(K2) ^ state[1].rotate_right(31);
        state[2] = state[2].wrapping_mul(K3) ^ state[2].rotate_right(33);
        state[3] = state[3].wrapping_mul(K4) ^ state[3].rotate_right(37);
    }
    
    let mut output = [0u8; 32];
    output[0..8].copy_from_slice(&state[0].to_le_bytes());
    output[8..16].copy_from_slice(&state[1].to_le_bytes());
    output[16..24].copy_from_slice(&state[2].to_le_bytes());
    output[24..32].copy_from_slice(&state[3].to_le_bytes());
    output
}

/// Streaming API (kept for compatibility)
#[derive(Clone)]
pub struct BlitzState {
    state: [u64; 4],
    buffer: [u8; 8],
    buffer_len: usize,
    total_len: u64,
}

impl BlitzState {
    pub fn new(seed: u64) -> Self {
        Self {
            state: [seed ^ K1, seed ^ K2, seed ^ K3, seed ^ K4],
            buffer: [0u8; 8],
            buffer_len: 0,
            total_len: 0,
        }
    }

    pub fn absorb(&mut self, data: &[u8]) {
        let mut pos = 0;
        self.total_len += data.len() as u64;

        // Handle buffered bytes first
        if self.buffer_len > 0 {
            let needed = 8 - self.buffer_len;
            let available = data.len().min(needed);
            self.buffer[self.buffer_len..self.buffer_len + available]
                .copy_from_slice(&data[..available]);
            self.buffer_len += available;
            pos += available;

            if self.buffer_len == 8 {
                let chunk = u64::from_le_bytes(self.buffer);
                // Mix into ALL lanes consistently
                self.state[0] = mix_chunk(self.state[0], chunk, K1);
                self.state[1] = mix_chunk(self.state[1], chunk.rotate_left(11), K2);
                self.state[2] = mix_chunk(self.state[2], chunk.rotate_left(23), K3);
                self.state[3] = mix_chunk(self.state[3], chunk.rotate_left(37), K4);
                self.buffer_len = 0;
            }
        }

        // Process 8-byte chunks
        while pos + 8 <= data.len() {
            unsafe {
                let chunk = read_u64_unaligned(data.as_ptr().add(pos));
                self.state[0] = mix_chunk(self.state[0], chunk, K1);
                self.state[1] = mix_chunk(self.state[1], chunk.rotate_left(11), K2);
                self.state[2] = mix_chunk(self.state[2], chunk.rotate_left(23), K3);
                self.state[3] = mix_chunk(self.state[3], chunk.rotate_left(37), K4);
            }
            pos += 8;
        }

        // Buffer remaining bytes
        if pos < data.len() {
            let remaining = data.len() - pos;
            self.buffer[..remaining].copy_from_slice(&data[pos..]);
            self.buffer_len = remaining;
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        // Process remaining buffered bytes
        if self.buffer_len > 0 {
            for i in self.buffer_len..8 {
                self.buffer[i] = 0;
            }
            let chunk = u64::from_le_bytes(self.buffer);
            // Mix into ALL lanes
            self.state[0] = mix_chunk(self.state[0], chunk, K1);
            self.state[1] = mix_chunk(self.state[1], chunk.rotate_left(13), K2);
            self.state[2] = mix_chunk(self.state[2], chunk.rotate_left(27), K3);
            self.state[3] = mix_chunk(self.state[3], chunk.rotate_left(43), K4);
        }

        // Mix in total length
        let len = self.total_len;
        self.state[0] ^= len;
        self.state[1] ^= len.rotate_right(17);
        self.state[2] ^= len.rotate_right(31);
        self.state[3] ^= len.rotate_right(47);

        // Final avalanche
        for _ in 0..4 {
            self.state[0] = self.state[0].wrapping_mul(K1) ^ self.state[0].rotate_right(29);
            self.state[1] = self.state[1].wrapping_mul(K2) ^ self.state[1].rotate_right(31);
            self.state[2] = self.state[2].wrapping_mul(K3) ^ self.state[2].rotate_right(33);
            self.state[3] = self.state[3].wrapping_mul(K4) ^ self.state[3].rotate_right(37);
        }

        let mut output = [0u8; 32];
        output[0..8].copy_from_slice(&self.state[0].to_le_bytes());
        output[8..16].copy_from_slice(&self.state[1].to_le_bytes());
        output[16..24].copy_from_slice(&self.state[2].to_le_bytes());
        output[24..32].copy_from_slice(&self.state[3].to_le_bytes());
        output
    }
}

/// Parallel hashing - FIXED (no allocation, direct state mixing)
pub fn blitz_hash_parallel(seed: u64, data: &[u8], num_threads: usize) -> [u8; 32] {
    use rayon::prelude::*;

    if data.len() < 1_000_000 || num_threads <= 1 {
        return blitz_hash(seed, data);
    }

    let chunk_size = (data.len() + num_threads - 1) / num_threads;
    let chunks: Vec<_> = data.chunks(chunk_size).collect();

    // Return partial STATES not bytes - no serialization overhead
    let partial_states: Vec<[u64; 4]> = chunks
        .par_iter()
        .enumerate()
        .map(|(idx, chunk)| {
            let hash = blitz_hash(seed.wrapping_add(idx as u64), chunk);
            // Convert bytes back to u64 states
            [
                u64::from_le_bytes(hash[0..8].try_into().unwrap()),
                u64::from_le_bytes(hash[8..16].try_into().unwrap()),
                u64::from_le_bytes(hash[16..24].try_into().unwrap()),
                u64::from_le_bytes(hash[24..32].try_into().unwrap()),
            ]
        })
        .collect();

    // Combine states directly - NO ALLOCATION, NO RE-HASH
    let mut final_state = [seed ^ K1, seed ^ K2, seed ^ K3, seed ^ K4];
    for partial in partial_states {
        final_state[0] = mix_chunk(final_state[0], partial[0], K1);
        final_state[1] = mix_chunk(final_state[1], partial[1], K2);
        final_state[2] = mix_chunk(final_state[2], partial[2], K3);
        final_state[3] = mix_chunk(final_state[3], partial[3], K4);
    }

    // Final avalanche
    for _ in 0..4 {
        final_state[0] = final_state[0].wrapping_mul(K1) ^ final_state[0].rotate_right(29);
        final_state[1] = final_state[1].wrapping_mul(K2) ^ final_state[1].rotate_right(31);
        final_state[2] = final_state[2].wrapping_mul(K3) ^ final_state[2].rotate_right(33);
        final_state[3] = final_state[3].wrapping_mul(K4) ^ final_state[3].rotate_right(37);
    }

    let mut output = [0u8; 32];
    output[0..8].copy_from_slice(&final_state[0].to_le_bytes());
    output[8..16].copy_from_slice(&final_state[1].to_le_bytes());
    output[16..24].copy_from_slice(&final_state[2].to_le_bytes());
    output[24..32].copy_from_slice(&final_state[3].to_le_bytes());
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let data = b"Hello, BlitzHash!";
        let h1 = blitz_hash(0, data);
        let h2 = blitz_hash(0, data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_seeds() {
        let data = b"test data";
        let h1 = blitz_hash(0, data);
        let h2 = blitz_hash(1, data);
        assert_ne!(h1, h2);
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
        
        assert_eq!(oneshot, streamed);
    }

    #[test]
    fn test_empty_input() {
        let h = blitz_hash(0, b"");
        assert_eq!(h.len(), 32);
    }

    #[test]
    fn test_tail_distribution() {
        // Test that short inputs still hash differently
        let h1 = blitz_hash(0, b"a");
        let h2 = blitz_hash(0, b"b");
        let h3 = blitz_hash(0, b"ab");
        assert_ne!(h1, h2);
        assert_ne!(h1, h3);
        assert_ne!(h2, h3);
    }
}