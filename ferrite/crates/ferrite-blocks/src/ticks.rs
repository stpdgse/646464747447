//! Scheduled ticks.
//!
//! A tick is identified by its position and type; scheduling one that is already pending does
//! nothing. Due ticks run ordered by `(trigger time, priority, scheduling order)`, the order the
//! vanilla server drains them in.

use std::collections::{BTreeMap, HashSet};
use std::hash::Hash;

use crate::pos::BlockPos;

/// Vanilla runs at most this many scheduled ticks of one kind per game tick.
pub const MAX_TICKS_PER_TICK: usize = 65536;

/// A pending tick, as reported by [`TickQueue::pending`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pending<T> {
    pub pos: BlockPos,
    pub kind: T,
    /// Game ticks until it runs (may be zero or negative if it is overdue).
    pub delay: i64,
    pub priority: i8,
}

/// Pending ticks, keyed for execution order.
///
/// Recordings of the official server show that it *saves* pending ticks in the order they were
/// scheduled, whatever their trigger times; [`TickQueue::pending`] reports them the same way so
/// saved tick lists can be compared verbatim.
#[derive(Clone, Debug)]
pub struct TickQueue<T> {
    // (trigger, priority, order) -> (pos, kind)
    queue: BTreeMap<(i64, i8, u64), (BlockPos, T)>,
    scheduled: HashSet<(BlockPos, T)>,
    next_order: u64,
}

impl<T> Default for TickQueue<T> {
    fn default() -> Self {
        Self { queue: BTreeMap::new(), scheduled: HashSet::new(), next_order: 0 }
    }
}

impl<T: Copy + Eq + Hash> TickQueue<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedules a tick to run at `trigger` unless one of the same kind is already pending at
    /// that position. Returns whether it was added.
    pub fn schedule(&mut self, pos: BlockPos, kind: T, trigger: i64, priority: i8) -> bool {
        if !self.scheduled.insert((pos, kind)) {
            return false;
        }
        self.queue.insert((trigger, priority, self.next_order), (pos, kind));
        self.next_order += 1;
        true
    }

    pub fn is_scheduled(&self, pos: BlockPos, kind: T) -> bool {
        self.scheduled.contains(&(pos, kind))
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Removes and returns the ticks due at `now`, in execution order. They are no longer
    /// pending, so they may reschedule themselves while they run.
    pub fn take_due(&mut self, now: i64, out: &mut Vec<(BlockPos, T)>) {
        out.clear();
        while out.len() < MAX_TICKS_PER_TICK {
            let Some(entry) = self.queue.first_entry() else { break };
            if entry.key().0 > now {
                break;
            }
            let item = entry.remove();
            self.scheduled.remove(&item);
            out.push(item);
        }
    }

    /// All pending ticks in the order they were scheduled (the order vanilla saves them in).
    pub fn pending(&self, now: i64) -> Vec<Pending<T>> {
        let mut all: Vec<_> = self.queue.iter().collect();
        all.sort_by_key(|(key, _)| key.2);
        all.into_iter()
            .map(|(&(trigger, priority, _), &(pos, kind))| Pending {
                pos,
                kind,
                delay: trigger - now,
                priority,
            })
            .collect()
    }

    /// Drops every pending tick at positions for which `keep` is false.
    pub fn retain(&mut self, mut keep: impl FnMut(BlockPos) -> bool) {
        let scheduled = &mut self.scheduled;
        self.queue.retain(|_, item| {
            let k = keep(item.0);
            if !k {
                scheduled.remove(item);
            }
            k
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i32) -> BlockPos {
        BlockPos::new(x, 0, 0)
    }

    #[test]
    fn duplicates_are_ignored_until_the_tick_has_been_taken() {
        let mut q = TickQueue::new();
        assert!(q.schedule(p(1), 7u8, 5, 0));
        assert!(!q.schedule(p(1), 7u8, 3, 0), "same position and kind");
        assert!(q.schedule(p(1), 8u8, 5, 0), "another kind is a different tick");
        let mut due = Vec::new();
        q.take_due(4, &mut due);
        assert!(due.is_empty());
        q.take_due(5, &mut due);
        assert_eq!(due, vec![(p(1), 7), (p(1), 8)]);
        assert!(q.schedule(p(1), 7u8, 9, 0), "may be rescheduled once taken");
    }

    #[test]
    fn order_is_time_then_priority_then_scheduling_order() {
        let mut q = TickQueue::new();
        q.schedule(p(1), 0u8, 10, 0);
        q.schedule(p(2), 0u8, 9, 1);
        q.schedule(p(3), 0u8, 10, -1);
        q.schedule(p(4), 0u8, 9, 1);
        q.schedule(p(5), 0u8, 9, 0);
        let mut due = Vec::new();
        q.take_due(10, &mut due);
        let xs: Vec<i32> = due.iter().map(|d| d.0.x).collect();
        assert_eq!(xs, vec![5, 2, 4, 3, 1]);
        assert!(q.is_empty());
    }

    #[test]
    fn overdue_ticks_still_run_and_report_their_delay() {
        let mut q = TickQueue::new();
        q.schedule(p(1), 0u8, 3, 0);
        assert_eq!(q.pending(5)[0].delay, -2);
        let mut due = Vec::new();
        q.take_due(100, &mut due);
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn execution_order_matches_a_sorted_model_under_random_use() {
        // Deterministic pseudo-random interleaving of schedule and take_due.
        let mut q = TickQueue::new();
        let mut model: Vec<((i64, i8, u64), i32)> = Vec::new();
        let (mut seed, mut order, mut now) = (12345u64, 0u64, 0i64);
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as i64
        };
        for round in 0..400 {
            for _ in 0..(next() % 6) {
                let x = (round * 10 + next() % 10) as i32;
                let (trigger, priority) = (now + 1 + next() % 9, (next() % 3 - 1) as i8);
                assert!(q.schedule(p(x), 0u8, trigger, priority) || model.iter().any(|m| m.1 == x));
                if !model.iter().any(|m| m.1 == x) {
                    model.push(((trigger, priority, order), x));
                }
                order += 1;
            }
            now += next() % 3;
            let mut due = Vec::new();
            q.take_due(now, &mut due);
            model.sort();
            let split = model.partition_point(|m| m.0.0 <= now);
            let want: Vec<i32> = model.drain(..split).map(|m| m.1).collect();
            assert_eq!(due.iter().map(|d| d.0.x).collect::<Vec<_>>(), want, "round {round}");
            assert_eq!(q.len(), model.len());
        }
    }

    #[test]
    fn retain_forgets_dropped_ticks_completely() {
        let mut q = TickQueue::new();
        q.schedule(p(1), 0u8, 3, 0);
        q.schedule(p(2), 0u8, 3, 0);
        q.retain(|pos| pos.x != 1);
        assert_eq!(q.len(), 1);
        assert!(q.schedule(p(1), 0u8, 4, 0));
    }
}
