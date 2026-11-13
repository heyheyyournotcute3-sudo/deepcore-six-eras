//! Inventories: stacks, slots, stacking and transfer rules.
//!
//! The layout is uniform: slots `0..9` are the hotbar, `9..45` the backpack.
//! Every mutation is expressed as a slot operation so that the network layer
//! can replay validated `MoveItems` intents against the authoritative copy.

use serde::{Deserialize, Serialize};

use crate::content::{ContentRegistry, ItemId};

/// Number of hotbar slots.
pub const HOTBAR_SLOTS: usize = 9;
/// Number of backpack slots.
pub const BACKPACK_SLOTS: usize = 36;
/// Total slots in a player inventory.
pub const INVENTORY_SLOTS: usize = HOTBAR_SLOTS + BACKPACK_SLOTS;

/// A stack of one item type.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ItemStack {
    /// Item type.
    pub item: ItemId,
    /// Stack size, always `>= 1`.
    pub count: u32,
}

impl ItemStack {
    /// Creates a stack.
    pub const fn new(item: ItemId, count: u32) -> Self {
        Self { item, count }
    }
}

/// A reference to a slot in an inventory.
pub type Slot = usize;

/// A player inventory: hotbar plus backpack.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    slots: Vec<Option<ItemStack>>,
    /// Index into the hotbar of the held item.
    pub selected: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    /// Creates an empty inventory.
    pub fn new() -> Self {
        Self {
            slots: vec![None; INVENTORY_SLOTS],
            selected: 0,
        }
    }

    /// Read-only access to a slot.
    pub fn get(&self, slot: Slot) -> Option<ItemStack> {
        self.slots.get(slot).copied().flatten()
    }

    /// All slots.
    pub fn slots(&self) -> &[Option<ItemStack>] {
        &self.slots
    }

    /// The item currently held in the hotbar.
    pub fn held(&self) -> Option<ItemStack> {
        self.get(self.selected as usize)
    }

    /// Total number of a given item across all slots.
    pub fn count(&self, item: ItemId) -> u32 {
        self.slots
            .iter()
            .flatten()
            .filter(|s| s.item == item)
            .map(|s| s.count)
            .sum()
    }

    /// True if every listed amount is present.
    pub fn has_all(&self, need: &[(ItemId, u32)]) -> bool {
        need.iter().all(|(item, n)| self.count(*item) >= *n)
    }

    /// Inserts items, stacking into existing stacks first (hotbar first, then
    /// backpack), then into empty slots. Returns the amount that did not fit.
    pub fn insert(&mut self, item: ItemId, count: u32, max_stack: u32) -> u32 {
        let mut left = count;
        // Pass 1: top up existing stacks.
        for slot in self.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if let Some(stack) = slot
                && stack.item == item
                && stack.count < max_stack
            {
                let room = max_stack - stack.count;
                let moved = room.min(left);
                stack.count += moved;
                left -= moved;
            }
        }
        // Pass 2: fill empty slots.
        for slot in self.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if slot.is_none() {
                let moved = max_stack.min(left);
                *slot = Some(ItemStack::new(item, moved));
                left -= moved;
            }
        }
        left
    }

    /// Removes up to `count` of `item` across the inventory. Returns how many
    /// were actually removed.
    pub fn remove(&mut self, item: ItemId, count: u32) -> u32 {
        let mut left = count;
        for slot in self.slots.iter_mut() {
            if left == 0 {
                break;
            }
            if let Some(stack) = slot
                && stack.item == item
            {
                let taken = stack.count.min(left);
                stack.count -= taken;
                left -= taken;
                if stack.count == 0 {
                    *slot = None;
                }
            }
        }
        count - left
    }

    /// Consumes a whole multi-item bill atomically: either everything is
    /// removed, or nothing is. Returns false if the inventory was short.
    pub fn consume_all(&mut self, need: &[(ItemId, u32)]) -> bool {
        if !self.has_all(need) {
            return false;
        }
        for (item, n) in need {
            self.remove(*item, *n);
        }
        true
    }

    /// Removes `count` items from one slot.
    pub fn take_from_slot(&mut self, slot: Slot, count: u32) -> Option<ItemStack> {
        let stack = self.get(slot)?;
        let taken = stack.count.min(count);
        if taken == 0 {
            return None;
        }
        let rest = stack.count - taken;
        if rest == 0 {
            self.slots[slot] = None;
        } else if let Some(s) = self.slots[slot].as_mut() {
            s.count = rest;
        }
        Some(ItemStack::new(stack.item, taken))
    }

    /// Moves items between two slots of this inventory, merging where possible.
    pub fn transfer(&mut self, from: Slot, to: Slot, count: u32, max_stack: u32) -> u32 {
        if from == to || from >= self.slots.len() || to >= self.slots.len() {
            return 0;
        }
        let Some(src) = self.slots[from] else {
            return 0;
        };
        let wanted = src.count.min(count);
        let dest = self.slots[to];
        let moved = match dest {
            None => {
                let m = wanted.min(max_stack);
                self.slots[to] = Some(ItemStack::new(src.item, m));
                m
            }
            Some(d) if d.item == src.item => {
                let room = max_stack.saturating_sub(d.count);
                let m = wanted.min(room);
                if let Some(dd) = self.slots[to].as_mut() {
                    dd.count += m;
                }
                m
            }
            Some(_) => 0,
        };
        if moved > 0 {
            let rest = src.count - moved;
            if rest == 0 {
                self.slots[from] = None;
            } else if let Some(s) = self.slots[from].as_mut() {
                s.count = rest;
            }
        }
        moved
    }

    /// Splits half of a stack into the first empty slot. Returns the target.
    pub fn split(&mut self, slot: Slot) -> Option<Slot> {
        let stack = self.get(slot)?;
        if stack.count < 2 {
            return None;
        }
        let empty = self.slots.iter().position(|s| s.is_none())?;
        let half = stack.count / 2;
        if let Some(s) = self.slots[slot].as_mut() {
            s.count -= half;
        }
        self.slots[empty] = Some(ItemStack::new(stack.item, half));
        Some(empty)
    }

    /// Moves a whole stack into the first matching/empty slot of the other
    /// range (quick-move). `to_hotbar` selects the destination range.
    pub fn quick_move(&mut self, slot: Slot, to_hotbar: bool, max_stack: u32) -> bool {
        let range = if to_hotbar {
            0..HOTBAR_SLOTS
        } else {
            HOTBAR_SLOTS..INVENTORY_SLOTS
        };
        let Some(stack) = self.get(slot) else {
            return false;
        };
        let mut remaining = stack.count;
        // Merge into same-item stacks first.
        for dst in range.clone() {
            if remaining == 0 {
                break;
            }
            if let Some(d) = self.get(dst)
                && d.item == stack.item
                && d.count < max_stack
            {
                let room = max_stack - d.count;
                let m = room.min(remaining);
                if let Some(dd) = self.slots[dst].as_mut() {
                    dd.count += m;
                }
                remaining -= m;
            }
        }
        for dst in range {
            if remaining == 0 {
                break;
            }
            if self.get(dst).is_none() {
                let m = remaining.min(max_stack);
                self.slots[dst] = Some(ItemStack::new(stack.item, m));
                remaining -= m;
            }
        }
        if remaining == 0 {
            self.slots[slot] = None;
            true
        } else if let Some(s) = self.slots[slot].as_mut() {
            s.count = remaining;
            remaining != stack.count
        } else {
            false
        }
    }

    /// Maximum stack size of an item from the registry.
    pub fn max_stack(reg: &ContentRegistry, item: ItemId) -> u32 {
        reg.item_def(item).map(|d| d.stack).unwrap_or(64)
    }
}

/// Convenience for building a bill of materials from string ids.
pub fn bill(reg: &ContentRegistry, items: &[(&str, u32)]) -> Option<Vec<(ItemId, u32)>> {
    items
        .iter()
        .map(|(id, n)| reg.item(id).map(|i| (i, *n)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(n: u16) -> ItemId {
        ItemId(n)
    }

    #[test]
    fn stacking_fills_existing_first() {
        let mut inv = Inventory::new();
        inv.slots[20] = Some(ItemStack::new(item(1), 10));
        let left = inv.insert(item(1), 5, 64);
        assert_eq!(left, 0);
        assert_eq!(inv.get(20).unwrap().count, 15);
    }

    #[test]
    fn insert_respects_stack_limits() {
        let mut inv = Inventory::new();
        let left = inv.insert(item(2), 100, 64);
        assert_eq!(left, 0);
        assert_eq!(inv.get(0).unwrap().count, 64);
        assert_eq!(inv.get(1).unwrap().count, 36);
    }

    #[test]
    fn insert_reports_overflow() {
        let mut inv = Inventory::new();
        for s in inv.slots.iter_mut() {
            *s = Some(ItemStack::new(item(3), 64));
        }
        assert_eq!(inv.insert(item(4), 10, 64), 10);
    }

    #[test]
    fn remove_clears_empty_slots() {
        let mut inv = Inventory::new();
        inv.insert(item(5), 5, 64);
        assert_eq!(inv.remove(item(5), 5), 5);
        assert!(inv.get(0).is_none());
    }

    #[test]
    fn consume_all_is_atomic() {
        let mut inv = Inventory::new();
        inv.insert(item(6), 3, 64);
        assert!(!inv.consume_all(&[(item(6), 4)]));
        assert_eq!(inv.count(item(6)), 3);
        assert!(inv.consume_all(&[(item(6), 2)]));
        assert_eq!(inv.count(item(6)), 1);
    }

    #[test]
    fn transfer_merges_and_rejects_mismatch() {
        let mut inv = Inventory::new();
        inv.slots[0] = Some(ItemStack::new(item(7), 40));
        inv.slots[1] = Some(ItemStack::new(item(7), 50));
        inv.slots[2] = Some(ItemStack::new(item(8), 1));
        assert_eq!(inv.transfer(0, 1, 40, 64), 14);
        assert_eq!(inv.get(0).unwrap().count, 26);
        assert_eq!(inv.transfer(0, 2, 10, 64), 0);
    }

    #[test]
    fn split_creates_half_stack() {
        let mut inv = Inventory::new();
        inv.slots[0] = Some(ItemStack::new(item(9), 11));
        let target = inv.split(0).unwrap();
        assert_eq!(inv.get(0).unwrap().count, 6);
        assert_eq!(inv.get(target).unwrap().count, 5);
    }

    #[test]
    fn quick_move_moves_between_ranges() {
        let mut inv = Inventory::new();
        inv.slots[0] = Some(ItemStack::new(item(10), 20));
        assert!(inv.quick_move(0, false, 64));
        assert!(inv.get(0).is_none());
        assert_eq!(inv.count(item(10)), 20);
    }
}
