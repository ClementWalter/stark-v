//! RIPEMD-160 hash implementation.
//!
//! This module provides a dependency-free RIPEMD-160 implementation suitable for
//! `no_std` environments.

use core::convert::TryInto;

const H0: [u32; 5] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476, 0xc3d2e1f0];

const R1: [usize; 80] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 7, 4, 13, 1, 10, 6, 15, 3, 12, 0, 9, 5,
    2, 14, 11, 8, 3, 10, 14, 4, 9, 15, 8, 1, 2, 7, 0, 6, 13, 11, 5, 12, 1, 9, 11, 10, 0, 8, 12, 4,
    13, 3, 7, 15, 14, 5, 6, 2, 4, 0, 5, 9, 7, 12, 2, 10, 14, 1, 3, 8, 11, 6, 15, 13,
];

const R2: [usize; 80] = [
    5, 14, 7, 0, 9, 2, 11, 4, 13, 6, 15, 8, 1, 10, 3, 12, 6, 11, 3, 7, 0, 13, 5, 10, 14, 15, 8, 12,
    4, 9, 1, 2, 15, 5, 1, 3, 7, 14, 6, 9, 11, 8, 12, 2, 10, 0, 4, 13, 8, 6, 4, 1, 3, 11, 15, 0, 5,
    12, 2, 13, 9, 7, 10, 14, 12, 15, 10, 4, 1, 5, 8, 7, 6, 2, 13, 14, 0, 3, 9, 11,
];

const S1: [u32; 80] = [
    11, 14, 15, 12, 5, 8, 7, 9, 11, 13, 14, 15, 6, 7, 9, 8, 7, 6, 8, 13, 11, 9, 7, 15, 7, 12, 15,
    9, 11, 7, 13, 12, 11, 13, 6, 7, 14, 9, 13, 15, 14, 8, 13, 6, 5, 12, 7, 5, 11, 12, 14, 15, 14,
    15, 9, 8, 9, 14, 5, 6, 8, 6, 5, 12, 9, 15, 5, 11, 6, 8, 13, 12, 5, 12, 13, 14, 11, 8, 5, 6,
];

const S2: [u32; 80] = [
    8, 9, 9, 11, 13, 15, 15, 5, 7, 7, 8, 11, 14, 14, 12, 6, 9, 13, 15, 7, 12, 8, 9, 11, 7, 7, 12,
    7, 6, 15, 13, 11, 9, 7, 15, 11, 8, 6, 6, 14, 12, 13, 5, 14, 13, 13, 7, 5, 15, 5, 8, 11, 14, 14,
    6, 14, 6, 9, 12, 9, 12, 5, 15, 8, 8, 5, 12, 9, 12, 5, 14, 6, 8, 13, 6, 5, 15, 13, 11, 11,
];

/// Compute RIPEMD-160 digest for the provided bytes.
pub fn ripemd160(data: &[u8]) -> [u8; 20] {
    let mut state = H0;

    for block in data.chunks_exact(64) {
        compress_block(&mut state, block);
    }

    let remainder = data.len() % 64;
    let mut tail = [0u8; 128];
    if remainder > 0 {
        tail[..remainder].copy_from_slice(&data[data.len() - remainder..]);
    }
    // We append the 0x80 bit first to ensure the padded message is unambiguous.
    tail[remainder] = 0x80;

    let bit_len = (data.len() as u64).wrapping_mul(8);
    let padded_len = if remainder + 1 + 8 <= 64 { 64 } else { 128 };
    let len_pos = padded_len - 8;
    // RIPEMD-160 encodes the length in little-endian for compatibility with MD-family hashing.
    tail[len_pos..len_pos + 8].copy_from_slice(&bit_len.to_le_bytes());

    compress_block(&mut state, &tail[..64]);
    if padded_len == 128 {
        compress_block(&mut state, &tail[64..128]);
    }

    let mut out = [0u8; 20];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    out
}

fn f(j: usize, x: u32, y: u32, z: u32) -> u32 {
    match j {
        0..=15 => x ^ y ^ z,
        16..=31 => (x & y) | (!x & z),
        32..=47 => (x | !y) ^ z,
        48..=63 => (x & z) | (y & !z),
        _ => x ^ (y | !z),
    }
}

fn k1(j: usize) -> u32 {
    match j {
        0..=15 => 0x00000000,
        16..=31 => 0x5a827999,
        32..=47 => 0x6ed9eba1,
        48..=63 => 0x8f1bbcdc,
        _ => 0xa953fd4e,
    }
}

fn k2(j: usize) -> u32 {
    match j {
        0..=15 => 0x50a28be6,
        16..=31 => 0x5c4dd124,
        32..=47 => 0x6d703ef3,
        48..=63 => 0x7a6d76e9,
        _ => 0x00000000,
    }
}

fn compress_block(state: &mut [u32; 5], block: &[u8]) {
    let mut w = [0u32; 16];
    for (i, chunk) in block.chunks_exact(4).take(16).enumerate() {
        w[i] = u32::from_le_bytes(chunk.try_into().expect("chunk size"));
    }

    let mut al = state[0];
    let mut bl = state[1];
    let mut cl = state[2];
    let mut dl = state[3];
    let mut el = state[4];

    let mut ar = state[0];
    let mut br = state[1];
    let mut cr = state[2];
    let mut dr = state[3];
    let mut er = state[4];

    for j in 0..80 {
        let t = al
            .wrapping_add(f(j, bl, cl, dl))
            .wrapping_add(w[R1[j]])
            .wrapping_add(k1(j));
        let t = t.rotate_left(S1[j]).wrapping_add(el);
        al = el;
        el = dl;
        dl = cl.rotate_left(10);
        cl = bl;
        // We update `bl` last to preserve the previous round value for rotation.
        bl = t;

        let t = ar
            .wrapping_add(f(79 - j, br, cr, dr))
            .wrapping_add(w[R2[j]])
            .wrapping_add(k2(j));
        let t = t.rotate_left(S2[j]).wrapping_add(er);
        ar = er;
        er = dr;
        dr = cr.rotate_left(10);
        cr = br;
        // We update `br` last to preserve the previous round value for rotation.
        br = t;
    }

    let t = state[1].wrapping_add(cl).wrapping_add(dr);
    state[1] = state[2].wrapping_add(dl).wrapping_add(er);
    state[2] = state[3].wrapping_add(el).wrapping_add(ar);
    state[3] = state[4].wrapping_add(al).wrapping_add(br);
    state[4] = state[0].wrapping_add(bl).wrapping_add(cr);
    state[0] = t;
}

#[cfg(test)]
mod tests {
    use super::ripemd160;

    #[test]
    fn test_ripemd160_empty() {
        let digest = ripemd160(b"");
        let expected = [
            0x9c, 0x11, 0x85, 0xa5, 0xc5, 0xe9, 0xfc, 0x54, 0x61, 0x28, 0x08, 0x97, 0x7e, 0xe8,
            0xf5, 0x48, 0xb2, 0x25, 0x8d, 0x31,
        ];
        assert_eq!(digest, expected);
    }

    #[test]
    fn test_ripemd160_abc() {
        let digest = ripemd160(b"abc");
        let expected = [
            0x8e, 0xb2, 0x08, 0xf7, 0xe0, 0x5d, 0x98, 0x7a, 0x9b, 0x04, 0x4a, 0x8e, 0x98, 0xc6,
            0xb0, 0x87, 0xf1, 0x5a, 0x0b, 0xfc,
        ];
        assert_eq!(digest, expected);
    }
}
