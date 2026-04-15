#![no_std]
use esp_rs_copro::lpbox::LPBox;
use core::option::Option;

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
    pub result : i32
}