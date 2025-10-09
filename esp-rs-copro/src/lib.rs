#![no_std]

pub trait MovableObject {
    fn move_to_main(&self) -> *mut u8;
    unsafe fn move_to_lp(&self) -> *mut u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
