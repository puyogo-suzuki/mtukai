use esp_rs_copro::{lpbox::LPBox, lpalloc, lpalloc::in_lp_mem_range, EspCoproError, collections::lpveccopy::LPVecCopy};
use rand::{Rng, SeedableRng, rngs::StdRng};

/// This is just tests for LPBox.
/// The common way is too far from these code.
/// Please refer to the examples for the common way to use LPBox.

#[derive(esp_rs_copro_procmacro::MovableObject)]
struct TestStruct {
    value1: i32,
    value2: i32
}

#[test]
fn test_lpbox_alloc() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let original = TestStruct { value1: 10, value2: 20 };
    let lpbox = LPBox::new(original);
    let lp_ptr = unsafe{ lpbox.get_moved_to_lp()? };
    assert!(lpalloc::in_lp_mem_range(lp_ptr.as_ptr()));
    Ok(())
}

#[test]
fn test_lpbox_transfer() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let original = TestStruct { value1: 10, value2: 20 };
    let lpbox = LPBox::new(original);
    let mut lp_ptr = unsafe{ lpbox.get_moved_to_lp()? };
    lp_ptr.value1 = 30;
    lp_ptr.value2 = 40;
    assert_eq!(lpbox.value1, 10);
    assert_eq!(lpbox.value2, 20);
    let moved = unsafe{ lp_ptr.get_moved_to_main()? };
    if lpbox.as_ptr() == moved.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(lpbox);
    }
    assert_eq!(moved.value1, 30);
    assert_eq!(moved.value2, 40);
    Ok(())
}

#[test]
fn test_addresstranslation_identical() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let original = TestStruct { value1: 10, value2: 20 };
    let lpbox = LPBox::new(original);
    let lp_ptr = unsafe{ lpbox.get_moved_to_lp()? };
    let moved = unsafe{ lp_ptr.get_moved_to_main()? };
    assert_eq!(lpbox.as_ptr(), moved.as_ptr());
    let _dont_drop = core::mem::ManuallyDrop::new(lpbox);
    Ok(())
}

#[test]
fn test_from_box() {
    let boxed = Box::new(TestStruct { value1: 50, value2: 60 });
    let lpboxed = LPBox::from_box(boxed);
    assert_eq!(lpboxed.value1, 50);
}

#[derive(esp_rs_copro_procmacro::MovableObject, PartialEq, Eq, Debug)]
struct TestLinkedList {
    next: Option<LPBox<TestLinkedList>>,
    val: i32
}

impl TestLinkedList {
    fn new(val: i32, next: Option<LPBox<TestLinkedList>>) -> Self {
        TestLinkedList { val, next }
    }
    fn copy(&self) -> Self {
        TestLinkedList {
            val: self.val,
            next: self.next.as_ref().map(|n| LPBox::new(n.copy()))
        }
    }
}

fn gen_random_linked_list(depth: u32, rng: &mut StdRng) -> TestLinkedList {
    let mut list = TestLinkedList::new(rng.next_u32() as i32, None);
    for _ in 0..depth {
        list = TestLinkedList::new(rng.next_u32() as i32, Some(LPBox::new(list)));
    }
    list
}

static RAND_SEED: [u8; 32] = [5u8; 32];

#[test]
fn test_linked_list_same_value() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let mut rng = StdRng::from_seed(RAND_SEED);
    let lpbox = LPBox::new(gen_random_linked_list(5, &mut rng));
    let lp_ptr = unsafe{ lpbox.get_moved_to_lp()? };
    assert_eq!(*lpbox, *lp_ptr);
    let moved = unsafe{ lp_ptr.get_moved_to_main()? };
    if lpbox.as_ptr() == moved.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(lpbox);
    }
    assert_eq!(*moved, *lp_ptr);
    Ok(())
}


#[test]
fn test_linked_list_correctly_moved() -> Result<(), EspCoproError> {
    fn check_is_in_lp_mem_range(is_in : bool, node : &LPBox<TestLinkedList>) {
        let mut cur = node;
        loop {
            assert!(in_lp_mem_range(cur.as_ptr()) == is_in);
            if let Some(cur_new) = &cur.next {
                cur = cur_new;
            } else {
                break;
            }
        }
    }

    lpalloc::lp_allocator_init();
    let mut rng = StdRng::from_seed(RAND_SEED);
    let lpbox = LPBox::new(gen_random_linked_list(5, &mut rng));
    let lp_ptr = unsafe{ lpbox.get_moved_to_lp()? };
    check_is_in_lp_mem_range(true, &lp_ptr);
    let moved = unsafe{ lp_ptr.get_moved_to_main()? };
    check_is_in_lp_mem_range(false, &moved);
    if lpbox.as_ptr() == moved.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(lpbox);
    }
    Ok(())
}

#[test]
fn test_linked_list_correctly_modified() -> Result<(), EspCoproError> {
    fn twice(node: &mut LPBox<TestLinkedList>) {
        let mut cur = node;
        loop {
            cur.val = cur.val.wrapping_mul(2);
            if let Some(next) = cur.next.as_mut() {
                cur = next;
            } else {
                break;
            }
        }
    }
    lpalloc::lp_allocator_init();
    let mut rng = StdRng::from_seed(RAND_SEED);
    let original = gen_random_linked_list(5, &mut rng);
    let lpbox = LPBox::new(original.copy());
    let mut lp_ptr = unsafe{ lpbox.get_moved_to_lp()? };
    twice(&mut lp_ptr);
    let moved = unsafe{ lp_ptr.get_moved_to_main()? };
    if lpbox.as_ptr() == moved.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(lpbox);
    }
    assert_eq!(*lp_ptr, *moved);
    assert_ne!(*moved, original);
    Ok(())
}

#[test]
fn test_lpvec_alloc() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let mut v = LPBox::new(LPVecCopy::new());
    v.push(10);
    v.push(40);
    v.push(20);
    assert!(!in_lp_mem_range((*v).as_ptr()));
    let moved = unsafe{ v.get_moved_to_lp()? };
    assert!(in_lp_mem_range((*moved).as_ptr()));
    let moved_back = unsafe{ moved.get_moved_to_main()? };
    assert!(!in_lp_mem_range((*moved_back).as_ptr()));
    if v.as_ptr() == moved_back.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(v);
    }
    Ok(())
}

#[test]
fn test_lpvec_correctly_moved() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let mut v = LPBox::new(LPVecCopy::new());
    v.push(10);
    v.push(40);
    v.push(20);
    let moved = unsafe{ v.get_moved_to_lp()? };
    assert_eq!(v, moved);
    let moved_back = unsafe{ moved.get_moved_to_main()? };
    assert_eq!(moved, moved_back);
    if v.as_ptr() == moved_back.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(v);
    }
    Ok(())
}

#[test]
fn test_lpvec_correctly_modified() -> Result<(), EspCoproError> {
    lpalloc::lp_allocator_init();
    let mut v = LPBox::new(LPVecCopy::new());
    v.push(10);
    v.push(20);
    v.push(30);
    let mut moved = unsafe{ v.get_moved_to_lp()? };
    moved[1] = 50;
    let moved_back = unsafe{ moved.get_moved_to_main()? };
    assert_eq!(moved_back[1], 50);
    if v.as_ptr() == moved_back.as_ptr() {
        let _dont_drop = core::mem::ManuallyDrop::new(v);
    }
    Ok(())
}

// TODO: check the exapansion of LPVec in the LP memory. It is hard to check the expansion with a current custom allocator.