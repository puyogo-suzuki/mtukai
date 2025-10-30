pub trait MovableObject {
    unsafe fn move_to_main(&self, dest : *mut u8);
    unsafe fn move_to_lp(&self, dest : *mut u8);
}