#![no_std]
use esp_rs_copro::{lpbox::LPBox, lpadapter::LPAdapter, lprc::LPRc};
use core::{option::Option, cell::RefCell};

#[derive(esp_rs_copro_procmacro::MovableObject)]
pub struct SimpleList {
    pub value : i32,
    pub next : Option<LPBox<SimpleList>>
}

impl SimpleList {
    pub fn new(value : i32, next : Option<LPBox<SimpleList>>) -> Self {
        SimpleList { value, next }
    }
    pub fn push(&mut self, value : i32) {
        match &mut self.next {
            Some(next) => next.push(value),
            None => self.next = Some(LPBox::new(SimpleList::new(value, None)))
        }
    }
    pub fn sum(&self) -> i32 {
        fn go(list : &SimpleList, res : i32) -> i32 {
            match &list.next {
                Some(next) => go(next, res + list.value),
                None => res + list.value
            }
        }
        go(self, 0)
    }
}

#[derive(esp_rs_copro_procmacro::MovableObject)]
pub struct MainLPParcel{
    pub data : LPBox<SimpleList>,
    pub rc_test: LPRc<RefCell<LPAdapter<i32>>>,
    pub rc_test2: LPRc<RefCell<LPAdapter<i32>>>,
    pub result : i32
}

impl MainLPParcel {
    pub fn new(data : LPBox<SimpleList>, result : i32) -> Self {
        let rc_test = esp_rs_copro::lprc::LPRc::new(core::cell::RefCell::new(LPAdapter::new(16)));
        let rc_test2 = rc_test.clone();
        MainLPParcel { data, rc_test, rc_test2, result }
    }

    pub fn do_rc_test(&self) {
        // rc_test and rc_test2 should point to the same underlying data, so we can modify one and see the change in the other.
        let rc_test_orig = *(self.rc_test.borrow());
        {
            let mut val = self.rc_test2.borrow_mut();
            *val += *rc_test_orig; // 16 + 16 = 32
        }
        let rc_test2_orig = *(self.rc_test2.borrow());
        {
            let mut val = self.rc_test.borrow_mut();
            *val += *rc_test2_orig; // 32 + 32 = 64
        }
    }

    pub fn test_after_lp(&self) -> bool {
        let rc_test_orig = *(self.rc_test.borrow());
        let rc_test2_orig = *(self.rc_test2.borrow());
        *rc_test_orig == 64 && *rc_test2_orig == 64 && self.rc_test.as_ptr() == self.rc_test2.as_ptr()
    }
}