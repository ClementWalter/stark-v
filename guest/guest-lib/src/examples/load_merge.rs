use serde::{Deserialize, Serialize};

#[repr(align(4))]
struct Aligned4([u8; 4]);

static DATA: Aligned4 = Aligned4([0x11, 0x22, 0x33, 0x44]);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadMergeResult {
    pub value: (u32, i16, i8),
}

pub fn load_merge() -> LoadMergeResult {
    LoadMergeResult {
        value: load_merge_impl(),
    }
}

#[inline(never)]
pub fn load_merge_impl() -> (u32, i16, i8) {
    unsafe {
        let base = DATA.0.as_ptr(); // *const u8, 4-byte aligned due to Aligned4

        // lw(base)
        let w = core::ptr::read_volatile(base as *const u32);

        // lh(base + 2)
        let h = core::ptr::read_volatile(base.add(2) as *const i16);

        // lb(base + 3)
        let b = core::ptr::read_volatile(base.add(3) as *const i8);

        (w, h, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_merge() {
        assert_eq!(
            load_merge(),
            LoadMergeResult {
                value: (0x44332211, 0x4433, 0x44)
            }
        );
    }
}
