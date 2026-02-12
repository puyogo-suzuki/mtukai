#[cfg(not(feature = "nottest"))]
use crate::lpalloc::lp_allocator_get_begin_and_end;

#[cfg(feature = "esp32c6")]
const LP_ADDRESS_MAX : usize = LP_ADDRESS_BASE + LP_ADDRESS_LEN;
#[cfg(feature = "esp32c6")]
const LP_ADDRESS_LEN : usize = 0x0004_0000;
#[cfg(feature = "esp32c6")]
const LP_ADDRESS_BASE : usize = 0x5000_0000;

#[cfg(not(feature = "nottest"))]
pub(crate) fn in_lp_range(addr : usize) -> bool {
    let (base, len) = lp_allocator_get_begin_and_end();
    addr.wrapping_sub(base) < len
}
#[cfg(feature = "esp32c6")]
pub(crate) fn in_lp_range(addr : usize) -> bool {
    (addr.wrapping_sub(LP_ADDRESS_BASE)) < LP_ADDRESS_LEN
}