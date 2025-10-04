//! BlitzHash - now writing for max speed
//! **WARNING: NOT CRYPTOGRAPHICALLY SECURE**

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

const K1: u64 = 0x517cc1b727220a95;
const K2: u64 = 0x85ebca6b2f3c8b51;
const K3: u64 = 0xc2b2ae3d27d4eb4f;
const K4: u64 = 0x165667b19e3779f9;

/// NUCLEAR mixing - inline everything, unroll manually
#[inline(always)]
fn mix_chunk(mut h: u64, chunk: u64, k: u64) -> u64 {
    h ^= chunk;
    h = h.wrapping_mul(k);
    h ^= h.rotate_right(27);
    h = h.wrapping_mul(K1);
    h ^= h.rotate_right(31);
    h
}

/// ultra fast baseline hash
pub fn blitz_hash(seed: u64, data: &[u8]) -> [u8; 32] {
    let mut state = [seed ^ K1, seed ^ K2, seed ^ K3, seed ^ K4];
    let mut pos = 0;
    
    // prrocesss the 32-byte chunks (4×8) - UNROLLED
    while pos + 32 <= data.len() {
        let c0 = u64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
        let c1 = u64::from_le_bytes(data[pos+8..pos+16].try_into().unwrap());
        let c2 = u64::from_le_bytes(data[pos+16..pos+24].try_into().unwrap());
        let c3 = u64::from_le_bytes(data[pos+24..pos+32].try_into().unwrap());
        
        state[0] = mix_chunk(state[0], c0, K1);
        state[1] = mix_chunk(state[1], c1, K2);
        state[2] = mix_chunk(state[2], c2, K3);
        state[3] = mix_chunk(state[3], c3, K4);
        
        pos += 32;
    }
    
    // prrrrocess the remaining 8-byte chunks
    while pos + 8 <= data.len() {
        let chunk = u64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
        state[0] = mix_chunk(state[0], chunk, K1);
        state[1] = mix_chunk(state[1], chunk, K2);
        pos += 8;
    }
    
    // Tail handling
    if pos < data.len() {
        let mut tail = [0u8; 8];
        tail[..data.len()-pos].copy_from_slice(&data[pos..]);
        let chunk = u64::from_le_bytes(tail);
        state[0] ^= chunk;
    }
    
    // Length mixing
    let len = data.len() as u64;
    state[0] ^= len;
    state[1] ^= len.rotate_right(17);
    state[2] ^= len.rotate_right(31);
    state[3] ^= len.rotate_right(47);
    
    // Final avalanche - AGGRESSIVE
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

/// SIMD accelerated version using AVX2
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn blitz_hash_avx2_inner(seed: u64, data: &[u8]) -> [u8; 32] {
    let mut state = [seed ^ K1, seed ^ K2, seed ^ K3, seed ^ K4];
    let mut pos = 0;
    
    // process 64 byte chunks with AVX2 (8×8 bytes)
    while pos + 64 <= data.len() {
        // Load 8 u64 values
        let ptr = data.as_ptr().add(pos) as *const __m256i;
        let v0 = _mm256_loadu_si256(ptr);
        let v1 = _mm256_loadu_si256(ptr.add(1));
        
        // extract values for mixing (not perfect SIMD but better than scalar)
        let mut chunks = [0u64; 8];
        _mm256_storeu_si256(chunks.as_mut_ptr() as *mut __m256i, v0);
        _mm256_storeu_si256(chunks.as_mut_ptr().add(4) as *mut __m256i, v1);
        
        // mix (unrolled)
        state[0] = mix_chunk(state[0], chunks[0], K1);
        state[1] = mix_chunk(state[1], chunks[1], K2);
        state[2] = mix_chunk(state[2], chunks[2], K3);
        state[3] = mix_chunk(state[3], chunks[3], K4);
        state[0] = mix_chunk(state[0], chunks[4], K1);
        state[1] = mix_chunk(state[1], chunks[5], K2);
        state[2] = mix_chunk(state[2], chunks[6], K3);
        state[3] = mix_chunk(state[3], chunks[7], K4);
        
        pos += 64;
    }

    while pos + 8 <= data.len() {
        let chunk = u64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
        state[0] = mix_chunk(state[0], chunk, K1);
        pos += 8;
    }
    
    // taill O-o-o-o-o-o
    if pos < data.len() {
        let mut tail = [0u8; 8];
        tail[..data.len()-pos].copy_from_slice(&data[pos..]);
        state[0] ^= u64::from_le_bytes(tail);
    }
    
    // Length + avalanche (same as baseline)
    let len = data.len() as u64;
    state[0] ^= len;
    state[1] ^= len.rotate_right(17);
    state[2] ^= len.rotate_right(31);
    state[3] ^= len.rotate_right(47);
    
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

/// Runtime-dispatched SIMD version
#[cfg(target_arch = "x86_64")]
pub fn blitz_hash_simd(seed: u64, data: &[u8]) -> [u8; 32] {
    if is_x86_feature_detected!("avx2") {
        unsafe { blitz_hash_avx2_inner(seed, data) }
    } else {
        blitz_hash(seed, data)
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn blitz_hash_simd(seed: u64, data: &[u8]) -> [u8; 32] {
    blitz_hash(seed, data)
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

        if self.buffer_len > 0 {
            let needed = 8 - self.buffer_len;
            let available = data.len().min(needed);
            self.buffer[self.buffer_len..self.buffer_len + available]
                .copy_from_slice(&data[..available]);
            self.buffer_len += available;
            pos += available;

            if self.buffer_len == 8 {
                let chunk = u64::from_le_bytes(self.buffer);
                self.state[0] = mix_chunk(self.state[0], chunk, K1);
                self.buffer_len = 0;
            }
        }

        while pos + 8 <= data.len() {
            let chunk = u64::from_le_bytes(data[pos..pos+8].try_into().unwrap());
            self.state[0] = mix_chunk(self.state[0], chunk, K1);
            self.state[1] = mix_chunk(self.state[1], chunk, K2);
            pos += 8;
        }

        if pos < data.len() {
            let remaining = data.len() - pos;
            self.buffer[..remaining].copy_from_slice(&data[pos..]);
            self.buffer_len = remaining;
        }
    }

    pub fn finalize(mut self) -> [u8; 32] {
        if self.buffer_len > 0 {
            for i in self.buffer_len..8 {
                self.buffer[i] = 0;
            }
            let chunk = u64::from_le_bytes(self.buffer);
            self.state[0] ^= chunk;
        }

        let len = self.total_len;
        self.state[0] ^= len;
        self.state[1] ^= len.rotate_right(17);
        self.state[2] ^= len.rotate_right(31);
        self.state[3] ^= len.rotate_right(47);

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

/// Parallel hashing - OPTIMIZED
pub fn blitz_hash_parallel(seed: u64, data: &[u8], num_threads: usize) -> [u8; 32] {
    use rayon::prelude::*;

    if data.len() < 1_000_000 || num_threads <= 1 {
        #[cfg(target_arch = "x86_64")]
        return blitz_hash_simd(seed, data);
        #[cfg(not(target_arch = "x86_64"))]
        return blitz_hash(seed, data);
    }

    let chunk_size = (data.len() + num_threads - 1) / num_threads;
    let chunks: Vec<_> = data.chunks(chunk_size).collect();

    let partial_hashes: Vec<[u8; 32]> = chunks
        .par_iter()
        .enumerate()
        .map(|(idx, chunk)| {
            #[cfg(target_arch = "x86_64")]
            return blitz_hash_simd(seed.wrapping_add(idx as u64), chunk);
            #[cfg(not(target_arch = "x86_64"))]
            return blitz_hash(seed.wrapping_add(idx as u64), chunk);
        })
        .collect();

    let mut combined = Vec::with_capacity(partial_hashes.len() * 32);
    for hash in partial_hashes {
        combined.extend_from_slice(&hash);
    }

    #[cfg(target_arch = "x86_64")]
    return blitz_hash_simd(seed, &combined);
    #[cfg(not(target_arch = "x86_64"))]
    return blitz_hash(seed, &combined);
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
    fn test_simd_matches_scalar() {
        let data = vec![0x42u8; 10000];
        let scalar = blitz_hash(0, &data);
        let simd = blitz_hash_simd(0, &data);
        assert_eq!(scalar, simd);
    }

    #[test]
    fn test_streaming() {
        let data = b"The quick brown fox";
        let oneshot = blitz_hash(42, data);
        let mut streaming = BlitzState::new(42);
        streaming.absorb(&data[..10]);
        streaming.absorb(&data[10..]);
        assert_eq!(oneshot, streaming.finalize());
    }
}
