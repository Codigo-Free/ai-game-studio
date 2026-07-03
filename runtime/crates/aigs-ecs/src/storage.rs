//! Type-erased component columns backing the [`World`](crate::World).

use std::any::Any;
use std::cell::RefCell;

/// One column per component type, indexed by entity slot. `RefCell` gives
/// per-column borrow checking so queries can borrow several columns at once.
pub(crate) struct Column<T: 'static> {
    pub(crate) cells: RefCell<Vec<Option<T>>>,
}

impl<T: 'static> Column<T> {
    pub(crate) fn new() -> Self {
        Self {
            cells: RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn set(&self, slot: usize, value: T) {
        let mut cells = self.cells.borrow_mut();
        if cells.len() <= slot {
            cells.resize_with(slot + 1, || None);
        }
        cells[slot] = Some(value);
    }
}

/// Object-safe view of a column, enough for entity teardown.
pub(crate) trait AnyColumn {
    fn as_any(&self) -> &dyn Any;
    /// Clears the component of a despawned entity slot.
    fn clear_slot(&self, slot: usize);
}

impl<T: 'static> AnyColumn for Column<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn clear_slot(&self, slot: usize) {
        if let Some(cell) = self.cells.borrow_mut().get_mut(slot) {
            *cell = None;
        }
    }
}
