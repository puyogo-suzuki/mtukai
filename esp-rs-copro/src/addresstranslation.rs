use alloc::collections::btree_map::BTreeMap;

pub(crate) struct AddressTranslationTable {
    main_to_lp: BTreeMap<usize, usize>,
    lp_to_main: BTreeMap<usize, usize>
}


impl AddressTranslationTable {
    pub const fn new() -> Self {
        AddressTranslationTable {
            main_to_lp: BTreeMap::new(),
            lp_to_main: BTreeMap::new()
        }
    }

    pub fn insert(&mut self, main: usize, lp: usize) {
        self.main_to_lp.insert(main, lp);
        self.lp_to_main.insert(lp, main);
    }

    pub fn get_by_main(&self, main: &usize) -> Option<&usize> {
        self.main_to_lp.get(main)
    }

    pub fn get_by_lp(&self, lp: &usize) -> Option<&usize> {
        self.lp_to_main.get(lp)
    }

    pub fn remove_by_main(&mut self, main: &usize) -> Option<usize> {
        self.main_to_lp.remove(main).and_then(|lp| {
            self.lp_to_main.remove(&lp);
            Some(lp)
        })
    }
    pub fn remove_by_lp(&mut self, lp: &usize) -> Option<usize> {
        self.lp_to_main.remove(lp).and_then(|main| {
            self.main_to_lp.remove(&main);
            Some(main)
        })
    }
}