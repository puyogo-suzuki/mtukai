use core::{alloc::Layout, mem, ptr};

#[cfg(feature = "nottest")]
use ::alloc::{alloc, boxed::Box, collections::btree_map::BTreeMap};
#[cfg(not(feature = "nottest"))]
use std::{alloc, collections::btree_map::BTreeMap};

// /// The combination of copy status and the address on the main memory.
// /// This is used for the return value of [`AddressTranslationTable::set_copied_by_lp`].
// pub(crate) struct SetCopiedByLpResult {
//     pub main_address : usize,
//     pub already_copied : bool
// }

/// This struct manages the translation between main and LP addresses.
/// To support the dropping of unsized values, it also stores the layout of the struct.
pub(crate) enum AddressTranslationAddressValue {
    // Droppable(usize, Box<dyn Fn(*mut u8)>),
    NonDroppable(usize, Layout)
}

impl AddressTranslationAddressValue {
    // Get the address.
    pub(crate) fn get_addr(&self) -> usize {
        match self {
            // AddressTranslationAddressValue::Droppable(addr, _) => addr,
            AddressTranslationAddressValue::NonDroppable(addr, _) => addr
        }.clone()
    }
    // Get the layout.
    pub(crate) fn get_layout(&self) -> Layout {
        match self {
            // AddressTranslationAddressValue::Droppable(addr, _) => addr,
            AddressTranslationAddressValue::NonDroppable(_, layout) => layout
        }.clone()
    }
}
/// The address translation item, which contains the address translation and the copy status.
pub(crate) struct AddressTranslationEntry {
    pub address : AddressTranslationAddressValue,
    pub copied : bool
}
/// The address translation table, which manages the translation between main and LP addresses.
pub(crate) struct AddressTranslationTable {
    main_to_lp: BTreeMap<usize, usize>,
    lp_to_main: BTreeMap<usize, AddressTranslationEntry>
}

impl AddressTranslationTable {
    /// Construct a translation table with no entries.
    pub(crate) const fn new() -> Self {
        AddressTranslationTable {
            main_to_lp: BTreeMap::new(),
            lp_to_main: BTreeMap::new()
        }
    }

    /// Insert a translation entry.
    /// This should be used by LPBox<T>, the owner of the value must be only one.
    pub(crate) fn insert<T : ?Sized>(&mut self, main: *mut T, lp: usize) {
        self.main_to_lp.insert(main as * const () as usize, lp);
        // if mem::needs_drop::<T>() {
        //     let foo = |v : *mut u8| unsafe{ptr::drop_in_place(v as *mut T)};
        //     self.lp_to_main.insert(lp, AddressTranslationEntry { address: AddressTranslationAddressValue::Droppable(main as usize, Box::new(foo)), copied: false });
        // } else {
            self.lp_to_main.insert(lp, AddressTranslationEntry { address: AddressTranslationAddressValue::NonDroppable(main as *mut () as usize, unsafe{Layout::for_value_raw(main)}), copied: false });
        // }
    }

    /// Insert a translation entry that will not be dropped by the translation table.
    /// This should be used by LPRc<T>, the owner of the value may be multiple.
    pub(crate) fn insert_no_drop<T : ?Sized>(&mut self, main: *mut T, lp: usize) {
        self.insert(main, lp);
        // self.main_to_lp.insert(main as * const () as usize, lp);
        // self.lp_to_main.insert(lp, AddressTranslationEntry { address: AddressTranslationAddressValue::NonDroppable(main as *mut () as usize, unsafe{Layout::for_value_raw(main)}), copied: false });
    }

    /// Get the LP address by the main address.
    pub(crate) fn get_by_main(&self, main: usize) -> Option<usize> {
        self.main_to_lp.get(&main).copied()
    }

    /// Get the main address and copy status by the LP address.
    pub(crate) fn get_by_lp(&self, lp: usize) -> Option<&AddressTranslationEntry> {
        self.lp_to_main.get(&lp)
    }

    // /// This must be called by LPRc and so on.
    // pub(crate) fn set_copied_by_lp(&mut self, lp: usize) -> Option<SetCopiedByLpResult>{
    //     if let Some(entry) = self.lp_to_main.get_mut(&lp) {
    //         let c = entry.copied;
    //         entry.copied = true;
    //         Some(SetCopiedByLpResult { main_address: entry.address.get_addr(), already_copied: c })
    //     } else {
    //         None
    //     }
    // }

    /// Removes the entry by lp address, returning the main address if it existed
    /// This must be called by only LPBox.
    pub(crate) fn remove_by_lp(&mut self, lp: usize) -> Option<(usize, Layout)> {
        self.lp_to_main.remove(&lp).and_then(|main_entry| {
            let addr = main_entry.address.get_addr();
            // do not free here, just return the address.
            self.main_to_lp.remove(&addr);
            Some((addr, main_entry.address.get_layout()))
        })
    }

    /// Clear the address translation table, dropping all the values on the main memory if needed.
    pub(crate) fn drop_and_clear(&mut self) {
        for e in self.lp_to_main.iter() {
            match &e.1.address {
                // AddressTranslationAddressValue::Droppable(addr, drop_fn) => {
                //     drop_fn(*addr as *mut u8);
                // },
                AddressTranslationAddressValue::NonDroppable(addr, layout) => {
                    unsafe {
                        alloc::dealloc(*addr as * mut u8, *layout);
                    }
                }
            }
        }
        self.lp_to_main.clear();
        self.main_to_lp.clear();
    }

    /// Remove the ently by the main address, returning the LP address if it existed.
    /// This must be called by only LPBox.
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