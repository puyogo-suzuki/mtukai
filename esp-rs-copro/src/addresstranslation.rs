use core::{alloc::Layout, mem, ptr};

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use crate::lpalloc;

pub(crate) struct SetCopiedByLpResult {
    pub main_address : usize,
    pub already_copied : bool
}

pub(crate) enum AddressTranslationAddressValue {
    Droppable(usize, Box<dyn Fn(*mut u8)>),
    NonDroppable(usize, Layout)
}

impl AddressTranslationAddressValue {
    pub fn get_addr(&self) -> usize {
        match self {
            AddressTranslationAddressValue::Droppable(addr, _) => addr,
            AddressTranslationAddressValue::NonDroppable(addr, _) => addr
        }.clone()
    }
}
pub(crate) struct AddressTranslationEntry {
    pub address : AddressTranslationAddressValue,
    pub copied : bool
}
pub(crate) struct AddressTranslationTable {
    main_to_lp: BTreeMap<usize, usize>,
    lp_to_main: BTreeMap<usize, AddressTranslationEntry>
}


impl AddressTranslationTable {
    pub const fn new() -> Self {
        AddressTranslationTable {
            main_to_lp: BTreeMap::new(),
            lp_to_main: BTreeMap::new()
        }
    }

    pub fn insert<T>(&mut self, main: *mut T, lp: usize) {
        println!("AddressTranslationTable::insert: main {:p} lp {:x}", main, lp);
        self.main_to_lp.insert(main as usize, lp);
        if mem::needs_drop::<T>() {
            let foo = |v : *mut u8| unsafe{ptr::drop_in_place(v as *mut T)};
            self.lp_to_main.insert(lp, AddressTranslationEntry { address: AddressTranslationAddressValue::Droppable(main as usize, Box::new(foo)), copied: false });
        } else {
            self.lp_to_main.insert(lp, AddressTranslationEntry { address: AddressTranslationAddressValue::NonDroppable(main as usize, unsafe{Layout::for_value_raw(main)}), copied: false });
        }
    }

    pub fn insert_no_drop<T>(&mut self, main: *mut T, lp: usize) {
        self.main_to_lp.insert(main as usize, lp);
        self.lp_to_main.insert(lp, AddressTranslationEntry { address: AddressTranslationAddressValue::NonDroppable(main as usize, unsafe{Layout::for_value_raw(main)}), copied: false });
    }

    pub fn get_by_main(&self, main: usize) -> Option<&usize> {
        self.main_to_lp.get(&main)
    }

    pub fn get_by_lp(&self, lp: usize) -> Option<&AddressTranslationEntry> {
        self.lp_to_main.get(&lp)
    }

    /// This must be called by LPRc and so on.
    pub fn set_copied_by_lp(&mut self, lp: usize) -> Option<SetCopiedByLpResult>{
        if let Some(entry) = self.lp_to_main.get_mut(&lp) {
            let c = entry.copied;
            entry.copied = true;
            Some(SetCopiedByLpResult { main_address: entry.address.get_addr(), already_copied: c })
        } else {
            None
        }
    }

    /// Removes the entry by lp address, returning the main address if it existed
    /// This must be called by only LPBox.
    pub fn remove_by_lp(&mut self, lp: usize) -> Option<usize> {
        self.lp_to_main.remove(&lp).and_then(|main_entry| {
            let addr = main_entry.address.get_addr();
            // do not free here, just return the address.
            self.main_to_lp.remove(&addr);
            Some(addr)
        })
    }

    pub fn drop_and_clear(&mut self) {
        for e in self.lp_to_main.iter() {
            println!("Dropping lp_to_main entry lp addr {:x}", e.1.address.get_addr());
            match &e.1.address {
                AddressTranslationAddressValue::Droppable(addr, drop_fn) => {
                    drop_fn(*addr as *mut u8);
                },
                AddressTranslationAddressValue::NonDroppable(addr, layout) => {
                    unsafe {
                        alloc::alloc::dealloc(*addr as * mut u8, *layout);
                    }
                }
            }
        }
        self.lp_to_main.clear();
        self.main_to_lp.clear();
    }

    pub(crate) fn remove_by_main(&mut self, main: usize) -> Option<usize> {
        self.main_to_lp.remove(&main).and_then(|lp| {
            self.lp_to_main.remove(&lp);
            Some(lp)
        })
    }
    // pub fn remove_by_lp(&mut self, lp: usize) -> Option<usize> {
    //     self.lp_to_main.remove(&lp).and_then(|main| {
    //         self.main_to_lp.remove(&main);
    //         Some(main)
    //     })
    // }
}