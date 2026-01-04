#[inline(always)]
pub const fn mask<const WIDTH: u32>() -> u64 {
    if WIDTH >= 64 {
        u64::MAX
    } else {
        (1u64 << WIDTH) - 1
    }
}

#[inline(always)]
pub const fn get<const SHIFT: u32, const WIDTH: u32>(value: u64) -> u64 {
    (value >> SHIFT) & mask::<WIDTH>()
}

#[inline(always)]
pub const fn set<const SHIFT: u32, const WIDTH: u32>(value: u64, bits: u64) -> u64 {
    let m = mask::<WIDTH>() << SHIFT;
    (value & !m) | ((bits & mask::<WIDTH>()) << SHIFT)
}

#[inline(always)]
pub const fn get_bit<const SHIFT: u32>(value: u64) -> bool {
    ((value >> SHIFT) & 1) != 0
}
