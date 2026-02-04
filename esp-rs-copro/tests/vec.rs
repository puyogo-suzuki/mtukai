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
        let mut tv = (LPVec::new(), LPVec::new());
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
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    let mut x: usize = 0;
    assert_eq!(v[x], 10);
    assert_eq!(v[x + 1], 20);
    x = x + 1;
    assert_eq!(v[x], 20);
    assert_eq!(v[x - 1], 10);
}

#[test]
fn test_debug_fmt() {
    let vec1: LPVec<LPAdapter<isize>> = LPVec::new();
    assert_eq!("[]", format!("{:?}", vec1));

    let vec2: LPVec<LPAdapter<isize>> = vec![0, 1].into();
    assert_eq!("[LPAdapter { inner: 0 }, LPAdapter { inner: 1 }]", format!("{:?}", vec2));
}

#[test]
fn test_push() {
    let mut v : LPVec<LPAdapter<isize>> = LPVec::new();
    v.push(1.into());
    assert_eq!(v, [1]);
    v.push(2.into());
    assert_eq!(v, [1, 2]);
    v.push(3.into());
    assert_eq!(v, [1, 2, 3]);
}

// #[test]
// fn test_extend() {
//     let mut v: LPVec<LPAdapter<i32>> = LPVec::new();
//     let mut w: LPVec<LPAdapter<i32>> = LPVec::new();

//     v.extend(w.clone());
//     assert_eq!(v, &[]);

//     v.extend(0..3);
//     for i in 0..3 {
//         w.push(i.into())
//     }

//     assert_eq!(v, w);

//     v.extend(3..10);
//     for i in 3..10 {
//         w.push(i.into())
//     }

//     assert_eq!(v, w);

//     v.extend(w.clone()); // specializes to `append`
//     assert!(v.iter().eq(w.iter().chain(w.iter())));

//     // Zero sized types
//     #[derive(PartialEq, Debug)]
//     struct Foo;
//     impl MovableObject for Foo {
//         unsafe fn move_to_main(&self, dest : *mut u8) {}
//         unsafe fn move_to_lp(&self, dest : *mut u8) {}
//     }

//     let mut a = LPVec::new();
//     let b = vec![Foo, Foo].into();

//     a.extend(b);
//     assert_eq!(a, &[Foo, Foo]);

//     // Double drop
//     let mut count_x = 0;
//     {
//         let mut x = LPVec::new();
//         let y : LPVec<DropCount> = vec![DropCount { count: &mut count_x }].into();
//         x.extend(y);
//     }
//     assert_eq!(count_x, 1);
// }

#[test]
fn test_extend_from_slice() {
    let a: LPVec<LPAdapter<isize>> = vec![1, 2, 3, 4, 5].into();
    let b: LPVec<LPAdapter<isize>> = vec![6, 7, 8, 9, 0].into();

    let mut v: LPVec<LPAdapter<isize>> = a;

    v.extend_from_slice(&b);

    assert_eq!(v, [1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
}

#[test]
fn test_extend_ref() {
    let mut v: LPVec<LPAdapter<isize>> = vec![1, 2].into();
    v.extend_from_slice(&[LPAdapter::from(3), LPAdapter::from(4), LPAdapter::from(5)]);

    assert_eq!(v.len(), 5);
    assert_eq!(v, [1, 2, 3, 4, 5]);

    let w: LPVec<LPAdapter<isize>> = vec![6, 7].into();
    v.extend_from_slice(&w);

    assert_eq!(v.len(), 7);
    assert_eq!(v, [1, 2, 3, 4, 5, 6, 7]);
}
