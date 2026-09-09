use std::cell::RefCell;

use esp_rs_copro::{lpalloc, lpbox::LPBox, lprc::LPRc, EspCoproError};

/// These tests exercise LPRc directly, in the same style as the LPBox integration tests.
/// The common transfer path is covered by the examples and LPBox tests.

#[derive(esp_rs_copro_procmacro::MovableObject, Debug, PartialEq, Eq)]
struct TestStruct {
    value1: i32,
    value2: i32,
}

#[derive(esp_rs_copro_procmacro::MovableObject)]
struct SharedPair {
    a: LPRc<TestStruct>,
    b: LPRc<TestStruct>,
}

#[test]
fn test_lprc_alloc() {
    lpalloc::lp_allocator_init();
    let rc = LPRc::new(TestStruct { value1: 10, value2: 20 });

    assert_eq!(rc.value1, 10);
    assert_eq!(rc.value2, 20);
    assert_eq!(rc.strong_count(), 1);
    assert_eq!(rc.weak_count(), 0);
}

#[test]
fn test_lprc_clone_and_drop() {
    lpalloc::lp_allocator_init();
    let rc = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let cloned = rc.clone();

    assert_eq!(rc.strong_count(), 2);
    assert!(LPRc::ptr_eq(&rc, &cloned));
    assert_eq!(*cloned, TestStruct { value1: 10, value2: 20 });

    drop(cloned);
    assert_eq!(rc.strong_count(), 1);
}

#[test]
fn test_lprc_raw_round_trip() {
    lpalloc::lp_allocator_init();
    let rc = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let original_ptr = rc.as_ptr();
    let raw = LPRc::into_raw(rc);
    let recovered = LPRc::from_raw(raw);

    assert_eq!(recovered.as_ptr(), original_ptr);
    assert_eq!(*recovered, TestStruct { value1: 10, value2: 20 });
}

#[test]
fn test_lprc_try_unwrap_requires_unique_strong_owner() {
    lpalloc::lp_allocator_init();
    let rc = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let clone = rc.clone();

    let rc = LPRc::try_unwrap(rc).expect_err("two strong owners must prevent unwrap");
    assert_eq!(rc.strong_count(), 2);
    drop(clone);
    assert_eq!(rc.strong_count(), 1);

    let value = LPRc::try_unwrap(rc).expect("the remaining strong owner can be unwrapped");
    assert_eq!(value, TestStruct { value1: 10, value2: 20 });
}

#[test]
fn test_lprc_weak_upgrade() {
    lpalloc::lp_allocator_init();
    let rc = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let weak = rc.downgrade();

    assert_eq!(rc.weak_count(), 1);
    assert_eq!(weak.strong_count(), 1);
    assert_eq!(weak.weak_count(), 1);

    let upgraded = weak.upgrade().expect("the strong reference is alive");
    assert_eq!(upgraded.strong_count(), 2);
    drop(upgraded);
    drop(rc);

    assert_eq!(weak.strong_count(), 0);
    assert!(weak.upgrade().is_none());
}

#[test]
fn test_lprc_weak_clone_and_drop() {
    lpalloc::lp_allocator_init();
    let rc = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let weak = rc.downgrade();
    let cloned = weak.clone();

    assert_eq!(rc.weak_count(), 2);
    assert_eq!(weak.weak_count(), 2);
    assert_eq!(cloned.weak_count(), 2);

    drop(cloned);
    assert_eq!(weak.weak_count(), 1);
    drop(rc);
    assert_eq!(weak.strong_count(), 0);
}

#[test]
fn test_lprc_new_lp() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let rc = unsafe { LPRc::new_lp(TestStruct { value1: 30, value2: 40 }) };

    assert!(lpalloc::in_lp_mem_range(rc.as_ptr()));
    assert_eq!(*rc, TestStruct { value1: 30, value2: 40 });
    Ok(())
}

#[test]
fn test_lprc_move_modify_move_back_and_preserve_pointer() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let original = LPBox::new(LPRc::new(RefCell::new(TestStruct {
        value1: 10,
        value2: 20,
    })));
    let original_value_ptr = original.as_ref().as_ptr();

    let moved = unsafe { original.get_moved_to_lp()? };
    assert!(lpalloc::in_lp_mem_range(moved.as_ref().as_ptr()));
    let moved_value: &RefCell<TestStruct> = &**moved;
    moved_value.borrow_mut().value1 = 30;
    moved_value.borrow_mut().value2 = 40;

    let moved_back = unsafe { moved.get_moved_to_main()? };
    let moved_back_value: &RefCell<TestStruct> = &**moved_back;
    assert_eq!(moved_back_value.borrow().value1, 30);
    assert_eq!(moved_back_value.borrow().value2, 40);
    assert_eq!(original_value_ptr, moved_back.as_ref().as_ptr());

    // The transfer copies the owner handle. Keep only the returned handle alive.
    let _original = core::mem::ManuallyDrop::new(original);
    let _moved = core::mem::ManuallyDrop::new(moved);
    Ok(())
}

#[test]
fn test_lprc_shared_pointer_survives_round_trip() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let shared = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let original = LPBox::new(SharedPair {
        a: shared.clone(),
        b: shared.clone(),
    });
    assert!(LPRc::ptr_eq(&original.a, &original.b));

    let moved = unsafe { original.get_moved_to_lp()? };
    assert!(LPRc::ptr_eq(&moved.a, &moved.b));

    let moved_back = unsafe { moved.get_moved_to_main()? };
    assert!(LPRc::ptr_eq(&moved_back.a, &moved_back.b));

    // The transfer copies owner handles. Keep the copied representations from dropping twice.
    let _shared = core::mem::ManuallyDrop::new(shared);
    let _original = core::mem::ManuallyDrop::new(original);
    let _moved = core::mem::ManuallyDrop::new(moved);
    let _moved_back = core::mem::ManuallyDrop::new(moved_back);
    Ok(())
}

#[test]
fn test_lprc_lp_replacement_separates_shared_pointers() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let a = LPRc::new(TestStruct { value1: 10, value2: 20 });
    let original = LPBox::new(SharedPair {
        a: a.clone(),
        b: a,
    });

    let mut moved = unsafe { original.get_moved_to_lp()? };
    assert!(LPRc::ptr_eq(&moved.a, &moved.b));
    moved.b = unsafe { LPRc::new_lp(TestStruct { value1: 30, value2: 40 }) };

    let moved_back = unsafe { moved.get_moved_to_main()? };
    assert_eq!(moved_back.a.strong_count(), 1);
    assert!(!LPRc::ptr_eq(&moved_back.a, &moved_back.b));
    assert_eq!(*moved_back.a, TestStruct { value1: 10, value2: 20 });
    assert_eq!(*moved_back.b, TestStruct { value1: 30, value2: 40 });

    // The transfer copies owner handles; keep the copied representations from dropping twice.
    let _original = core::mem::ManuallyDrop::new(original);
    let _moved = core::mem::ManuallyDrop::new(moved);
    let _moved_back = core::mem::ManuallyDrop::new(moved_back);
    Ok(())
}
