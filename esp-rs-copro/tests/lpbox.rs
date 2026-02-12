use esp_rs_copro::{lpbox::LPBox, lpalloc};

#[derive(esp_rs_copro_procmacro::MovableObject)]
struct TestStruct {
    value1: i32,
    value2: i32
}


#[test]
fn test_lpbox_alloc() {
    lpalloc::lp_allocator_init();
    let original = TestStruct { value1: 10, value2: 20 };
    let mut lpbox = LPBox::new(original);
    let lp_ptr = lpbox.get_moved_to_lp();
}