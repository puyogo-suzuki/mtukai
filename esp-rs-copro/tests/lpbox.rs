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

#[test]
fn test_lpbox_transfer() {
    lpalloc::lp_allocator_init();
    let original = TestStruct { value1: 10, value2: 20 };
    let mut lpbox = LPBox::new(original);
    let mut lp_ptr = lpbox.get_moved_to_lp();
    lp_ptr.value1 = 30;
    lp_ptr.value2 = 40;
    assert_eq!(lpbox.value1, 10);
    assert_eq!(lpbox.value2, 20);
    let mut moved = lp_ptr.get_moved_to_main();
    if lpbox.as_ptr() == moved.as_ptr() {
        let dont_drop = core::mem::ManuallyDrop::new(lpbox);
    }
    assert_eq!(moved.value1, 30);
    assert_eq!(moved.value2, 40);
}