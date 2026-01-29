use esp_rs_copro::{collections::lpvec::LPVec, movableobject::MovableObject, lpadapter::LPAdapter};
use std::ptr;

#[test]
fn test_vec_small() {
    assert_eq!(size_of::<LPVec<LPAdapter<u8>>>(), size_of::<usize>() * 3)
}

struct DropCount<'a> {
    count : &'a mut usize
}
impl Drop for DropCount<'_> {
    fn drop(&mut self) {
        *(self.count) += 1;
    }
}
impl MovableObject for DropCount<'_> {
    unsafe fn move_to_main(&self, dest : *mut u8) {}
    unsafe fn move_to_lp(&self, dest : *mut u8) {}
}

#[test]
fn test_double_drop() {
    let (mut count_x, mut count_y) = (0, 0);
    {
        let mut tv = (Vec::new(), Vec::new());
        tv.0.push(DropCount { count: &mut count_x });
        tv.1.push(DropCount { count: &mut count_y });

        drop(tv.0);
    }

    assert_eq!(count_x, 1);
    assert_eq!(count_y, 1);
}

#[test]
fn test_reserve() {
    let mut v  : LPVec<LPAdapter<i32>> = LPVec::new();
    assert_eq!(v.capacity(), 0);
    v.reserve(10);
    assert!(v.capacity() >= 10);

    for i in 0..16 {
        v.push(i.into());
    }
    assert!(v.capacity() >= 16);
    v.reserve(16);
    assert!(v.capacity() >= 32);
    v.push(16.into());
    v.reserve(16);
    assert!(v.capacity() >= 33);
}

#[test]
fn test_zst_capacity() {
    assert_eq!(LPVec::<LPAdapter<()>>::new().capacity(), usize::MAX);
}

#[test]
fn test_indexing() {
    let v: LPVec<LPAdapter<isize>> = vec![10, 20].into();
    assert_eq!(v[0], 10.into());
    assert_eq!(v[1], 20.into());
    let mut x: usize = 0;
    assert_eq!(v[x], 10.into());
    assert_eq!(v[x + 1], 20.into());
    x = x + 1;
    assert_eq!(v[x], 20.into());
    assert_eq!(v[x - 1], 10.into());
}

#[test]
fn test_debug_fmt() {
    let vec1: LPVec<LPAdapter<isize>> = LPVec::new();
    assert_eq!("[]", format!("{:?}", vec1));

    let vec2: LPVec<LPAdapter<isize>> = vec![0, 1].into();
    assert_eq!("[LPAdapter { inner: 0 }, LPAdapter { inner: 1 }]", format!("{:?}", vec2));
}
