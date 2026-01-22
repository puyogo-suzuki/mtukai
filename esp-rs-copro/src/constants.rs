#[cfg(any(feature = "esp32c6", test))]
const LP_ADDRESS_MAX : usize = LP_ADDRESS_BASE + LP_ADDRESS_LEN;
#[cfg(feature = "esp32c6")]
const LP_ADDRESS_LEN : usize = 0x0004_0000;
#[cfg(feature = "esp32c6")]
const LP_ADDRESS_BASE : usize = 0x5000_0000;

// TODO: TO BE REMOVED ...
#[cfg(test)]
const LP_ADDRESS_LEN : usize = 0x0004_0000;
#[cfg(test)]
const LP_ADDRESS_BASE : usize = 0x5000_0000;

#[cfg(any(feature = "esp32c6", test))]
pub(crate) fn in_lp_range(addr : usize) -> bool {
    (addr.wrapping_sub(LP_ADDRESS_BASE)) < LP_ADDRESS_LEN
}
#[cfg(not(any(feature = "esp32c6", test)))]
pub(crate) fn in_lp_range(_addr : usize) -> bool {
    false
}