use std::cell::{Cell, RefCell};
use std::fmt;

use crate::Error;
use crate::game::{BlockPos, Dimension};
use crate::host;
use crate::server::world::MAX_CHANGES_PER_CALL;
use crate::server::{BlockState, Player};

pub const DEFAULT_EDITS_PER_TICK: usize = 4096;
pub const MAX_EDITS_PER_TICK: usize = 16384;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EditId(u64);

impl fmt::Display for EditId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progress {
    pub done: u64,
    pub total: u64,
}

impl Progress {
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return 100;
        }
        (self.done.min(self.total) * 100 / self.total) as u8
    }
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    palette: Vec<BlockState>,
    blocks: Vec<(BlockPos, u32)>,
    last: u32,
}

impl Snapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (BlockPos, &BlockState)> {
        self.blocks
            .iter()
            .map(|&(pos, index)| (pos, &self.palette[index as usize]))
    }

    pub fn push(&mut self, pos: BlockPos, state: impl Into<BlockState>) {
        let state = state.into();
        let index = self.index_of(state);
        self.blocks.push((pos, index));
    }

    fn index_of(&mut self, state: BlockState) -> u32 {
        let text = state.as_str();
        if self
            .palette
            .get(self.last as usize)
            .is_some_and(|last| last.as_str() == text)
        {
            return self.last;
        }
        let found = self
            .palette
            .iter()
            .position(|known| known.as_str().len() == text.len() && known.as_str() == text);
        let index = match found {
            Some(index) => index as u32,
            None => {
                self.palette.push(state);
                (self.palette.len() - 1) as u32
            }
        };
        self.last = index;
        index
    }
}

impl<S: Into<BlockState>> FromIterator<(BlockPos, S)> for Snapshot {
    fn from_iter<I: IntoIterator<Item = (BlockPos, S)>>(changes: I) -> Self {
        let mut snapshot = Self::new();
        for (pos, state) in changes {
            snapshot.push(pos, state);
        }
        snapshot
    }
}

type Change = Box<dyn FnMut(BlockPos, &BlockState) -> Option<BlockState>>;

enum Work {
    Fill {
        area: Area,
        state: BlockState,
    },
    Replace {
        area: Area,
        from: BlockState,
        exact: bool,
        to: BlockState,
    },
    Map {
        area: Area,
        change: Change,
    },
    Restore {
        snapshot: Snapshot,
        next: usize,
    },
}

pub struct Edit {
    dimension: Dimension,
    work: Work,
    record: bool,
    player: Option<Player>,
}

impl Edit {
    pub fn fill(
        dimension: &Dimension,
        a: BlockPos,
        b: BlockPos,
        state: impl Into<BlockState>,
    ) -> Self {
        Self::new(
            dimension,
            Work::Fill {
                area: Area::new(a, b),
                state: state.into(),
            },
        )
    }

    pub fn replace(
        dimension: &Dimension,
        a: BlockPos,
        b: BlockPos,
        from: impl Into<BlockState>,
        to: impl Into<BlockState>,
    ) -> Self {
        let from = from.into();
        let exact = from.as_str().ends_with(']');
        Self::new(
            dimension,
            Work::Replace {
                area: Area::new(a, b),
                from,
                exact,
                to: to.into(),
            },
        )
    }

    pub fn map(
        dimension: &Dimension,
        a: BlockPos,
        b: BlockPos,
        change: impl FnMut(BlockPos, &BlockState) -> Option<BlockState> + 'static,
    ) -> Self {
        Self::new(
            dimension,
            Work::Map {
                area: Area::new(a, b),
                change: Box::new(change),
            },
        )
    }

    pub fn set<S: Into<BlockState>>(
        dimension: &Dimension,
        changes: impl IntoIterator<Item = (BlockPos, S)>,
    ) -> Self {
        Self::restore(dimension, changes.into_iter().collect())
    }

    pub fn restore(dimension: &Dimension, snapshot: Snapshot) -> Self {
        Self::new(dimension, Work::Restore { snapshot, next: 0 })
    }

    fn new(dimension: &Dimension, work: Work) -> Self {
        Self {
            dimension: dimension.clone(),
            work,
            record: false,
            player: None,
        }
    }

    pub fn by(mut self, player: &Player) -> Self {
        self.player = Some(player.clone());
        self
    }

    pub fn record_undo(mut self) -> Self {
        self.record = true;
        self
    }

    pub fn size(&self) -> u64 {
        match &self.work {
            Work::Fill { area, .. } | Work::Replace { area, .. } | Work::Map { area, .. } => {
                area.remaining()
            }
            Work::Restore { snapshot, next } => (snapshot.len() - next) as u64,
        }
    }

    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    pub fn queue(self) -> Result<EditId, Error> {
        let per_tick = PER_TICK.with(Cell::get).ok_or_else(|| {
            Error::new(
                "queued edits need hooks.queue_edits(blocks_per_tick) or hooks.on_edit_done(..) in init",
            )
        })?;
        let id = EditId(NEXT_ID.with(|next| {
            let id = next.get();
            next.set(id + 1);
            id
        }));
        let mut job = Job::new(id, self);
        let budget = per_tick.saturating_sub(USED.with(Cell::get));
        if budget > 0 {
            let used = job.step(budget).map_err(Error::new)?;
            USED.with(|u| u.set(u.get() + used));
        }
        if job.is_done() {
            FINISHED.with(|f| f.borrow_mut().push(job.finish(false, None)));
        } else {
            JOBS.with(|jobs| jobs.borrow_mut().push(job));
        }
        Ok(id)
    }
}

impl fmt::Debug for Edit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Edit")
            .field("dimension", &self.dimension)
            .field("size", &self.size())
            .field("record", &self.record)
            .finish()
    }
}

#[derive(Debug)]
pub struct EditDone {
    pub id: EditId,
    pub dimension: Dimension,
    pub player: Option<Player>,
    pub changed: u64,
    pub cancelled: bool,
    pub error: Option<String>,
    undo: Option<Snapshot>,
}

impl EditDone {
    pub fn undo_size(&self) -> usize {
        self.undo.as_ref().map_or(0, Snapshot::len)
    }

    pub fn take_undo(&mut self) -> Option<Edit> {
        let snapshot = self.undo.take()?;
        let mut edit = Edit::restore(&self.dimension, snapshot);
        edit.player = self.player.clone();
        Some(edit)
    }
}

pub fn progress(id: EditId) -> Option<Progress> {
    JOBS.with(|jobs| {
        jobs.borrow()
            .iter()
            .find(|job| job.id == id)
            .map(|job| Progress {
                done: job.done,
                total: job.total,
            })
    })
}

pub fn cancel(id: EditId) -> bool {
    let job = JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        let index = jobs.iter().position(|job| job.id == id)?;
        Some(jobs.remove(index))
    });
    match job {
        Some(job) => {
            FINISHED.with(|f| f.borrow_mut().push(job.finish(true, None)));
            true
        }
        None => false,
    }
}

pub fn pending_edits() -> usize {
    JOBS.with(|jobs| jobs.borrow().len())
}

thread_local! {
    static PER_TICK: Cell<Option<usize>> = const { Cell::new(None) };
    static USED: Cell<usize> = const { Cell::new(0) };
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static JOBS: RefCell<Vec<Job>> = const { RefCell::new(Vec::new()) };
    static FINISHED: RefCell<Vec<EditDone>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn configure(per_tick: Option<usize>) {
    PER_TICK.with(|p| p.set(per_tick));
    USED.with(|u| u.set(0));
    JOBS.with(|jobs| jobs.borrow_mut().clear());
    FINISHED.with(|f| f.borrow_mut().clear());
}

pub(crate) fn run_tick() {
    USED.with(|u| u.set(0));
    let Some(per_tick) = PER_TICK.with(Cell::get) else {
        return;
    };
    let mut jobs = JOBS.with(|jobs| std::mem::take(&mut *jobs.borrow_mut()));
    let mut budget = per_tick;
    let mut finished = Vec::new();
    let mut index = 0;
    while index < jobs.len() && budget > 0 {
        let share = (budget / (jobs.len() - index)).max(1);
        let job = &mut jobs[index];
        match job.step(share) {
            Ok(used) => budget = budget.saturating_sub(used),
            Err(error) => {
                finished.push(jobs.remove(index).finish(false, Some(error)));
                continue;
            }
        }
        if jobs[index].is_done() {
            finished.push(jobs.remove(index).finish(false, None));
        } else {
            index += 1;
        }
    }
    USED.with(|u| u.set(per_tick - budget));
    JOBS.with(|queued| {
        let mut queued = queued.borrow_mut();
        jobs.append(&mut queued);
        *queued = jobs;
    });
    FINISHED.with(|f| f.borrow_mut().append(&mut finished));
}

pub(crate) fn take_finished() -> Vec<EditDone> {
    FINISHED.with(|f| std::mem::take(&mut *f.borrow_mut()))
}

struct Job {
    id: EditId,
    dimension: Dimension,
    work: Work,
    undo: Option<Snapshot>,
    player: Option<Player>,
    changed: u64,
    done: u64,
    total: u64,
}

impl Job {
    fn new(id: EditId, edit: Edit) -> Self {
        let total = edit.size();
        Self {
            id,
            dimension: edit.dimension,
            work: edit.work,
            undo: edit.record.then(Snapshot::new),
            player: edit.player,
            changed: 0,
            done: 0,
            total,
        }
    }

    fn is_done(&self) -> bool {
        self.done >= self.total
    }

    fn finish(self, cancelled: bool, error: Option<String>) -> EditDone {
        EditDone {
            id: self.id,
            dimension: self.dimension,
            player: self.player,
            changed: self.changed,
            cancelled,
            error,
            undo: self.undo,
        }
    }

    fn step(&mut self, budget: usize) -> Result<usize, String> {
        let Self {
            dimension,
            work,
            undo,
            changed,
            done,
            ..
        } = self;
        let dimension = dimension.id();
        let mut batch = Batch::new(dimension, budget);
        let processed = match work {
            Work::Fill { area, state } => {
                let target = state.as_str();
                let mut processed = 0;
                for pos in area.by_ref().take(budget) {
                    processed += 1;
                    if let Some(undo) = undo.as_mut() {
                        let Some(old) = read(dimension, pos) else {
                            continue;
                        };
                        if old.as_str() == target {
                            continue;
                        }
                        undo.push(pos, old);
                    }
                    batch.push(pos, target)?;
                }
                processed
            }
            Work::Replace {
                area,
                from,
                exact,
                to,
            } => {
                let target = to.as_str();
                let mut processed = 0;
                for pos in area.by_ref().take(budget) {
                    processed += 1;
                    let Some(old) = read(dimension, pos) else {
                        continue;
                    };
                    if !matches(&old, from, *exact) || old.as_str() == target {
                        continue;
                    }
                    if let Some(undo) = undo.as_mut() {
                        undo.push(pos, old);
                    }
                    batch.push(pos, target)?;
                }
                processed
            }
            Work::Map { area, change } => {
                let mut owned: Vec<(BlockPos, BlockState)> =
                    Vec::with_capacity(budget.min(MAX_CHANGES_PER_CALL));
                let mut processed = 0;
                for pos in area.by_ref().take(budget) {
                    processed += 1;
                    let Some(old) = read(dimension, pos) else {
                        continue;
                    };
                    let Some(new) = change(pos, &old).filter(|new| *new != old) else {
                        continue;
                    };
                    if let Some(undo) = undo.as_mut() {
                        undo.push(pos, old);
                    }
                    owned.push((pos, new));
                    if owned.len() == MAX_CHANGES_PER_CALL {
                        *changed += send_owned(dimension, &mut owned)?;
                    }
                }
                *changed += send_owned(dimension, &mut owned)?;
                processed
            }
            Work::Restore { snapshot, next } => {
                let end = (*next + budget).min(snapshot.blocks.len());
                for &(pos, index) in &snapshot.blocks[*next..end] {
                    let target = snapshot.palette[index as usize].as_str();
                    if let Some(undo) = undo.as_mut() {
                        let Some(old) = read(dimension, pos) else {
                            continue;
                        };
                        if old.as_str() == target {
                            continue;
                        }
                        undo.push(pos, old);
                    }
                    batch.push(pos, target)?;
                }
                let processed = end - *next;
                *next = end;
                processed
            }
        };
        *changed += batch.flush()?;
        *done += processed as u64;
        Ok(processed)
    }
}

fn read(dimension: &str, pos: BlockPos) -> Option<BlockState> {
    host::server_get_block(dimension, pos).map(BlockState::from_host)
}

fn matches(old: &BlockState, from: &BlockState, exact: bool) -> bool {
    let (old, from) = (old.as_str().as_bytes(), from.as_str().as_bytes());
    if exact {
        return old == from;
    }
    old.len() >= from.len()
        && old[..from.len()] == *from
        && (old.len() == from.len() || old[from.len()] == b'[')
}

fn send_owned(dimension: &str, owned: &mut Vec<(BlockPos, BlockState)>) -> Result<u64, String> {
    if owned.is_empty() {
        return Ok(0);
    }
    let changes: Vec<(BlockPos, &str)> = owned
        .iter()
        .map(|(pos, state)| (*pos, state.as_str()))
        .collect();
    host::server_set_blocks(dimension, &changes)?;
    let sent = owned.len() as u64;
    owned.clear();
    Ok(sent)
}

struct Batch<'a> {
    dimension: &'a str,
    changes: Vec<(BlockPos, &'a str)>,
    sent: u64,
}

impl<'a> Batch<'a> {
    fn new(dimension: &'a str, budget: usize) -> Self {
        Self {
            dimension,
            changes: Vec::with_capacity(budget.min(MAX_CHANGES_PER_CALL)),
            sent: 0,
        }
    }

    fn push(&mut self, pos: BlockPos, state: &'a str) -> Result<(), String> {
        self.changes.push((pos, state));
        if self.changes.len() == MAX_CHANGES_PER_CALL {
            self.send()?;
        }
        Ok(())
    }

    fn send(&mut self) -> Result<(), String> {
        if !self.changes.is_empty() {
            host::server_set_blocks(self.dimension, &self.changes)?;
            self.sent += self.changes.len() as u64;
            self.changes.clear();
        }
        Ok(())
    }

    fn flush(mut self) -> Result<u64, String> {
        self.send()?;
        Ok(self.sent)
    }
}

#[derive(Clone, Copy, Debug)]
struct Area {
    min: BlockPos,
    max: BlockPos,
    next: Option<BlockPos>,
}

impl Area {
    fn new(a: BlockPos, b: BlockPos) -> Self {
        let min = BlockPos::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
        let max = BlockPos::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
        Self {
            min,
            max,
            next: Some(min),
        }
    }

    fn remaining(&self) -> u64 {
        let Some(next) = self.next else {
            return 0;
        };
        let width = u64::from(self.min.x.abs_diff(self.max.x)) + 1;
        let depth = u64::from(self.min.z.abs_diff(self.max.z)) + 1;
        let layers_after = u64::from(next.y.abs_diff(self.max.y));
        let rows_after = u64::from(next.z.abs_diff(self.max.z));
        let in_row = u64::from(next.x.abs_diff(self.max.x)) + 1;
        layers_after * width * depth + rows_after * width + in_row
    }
}

impl Iterator for Area {
    type Item = BlockPos;

    fn next(&mut self) -> Option<BlockPos> {
        let pos = self.next?;
        let mut next = pos;
        if next.x < self.max.x {
            next.x += 1;
        } else if next.z < self.max.z {
            next.x = self.min.x;
            next.z += 1;
        } else if next.y < self.max.y {
            next.x = self.min.x;
            next.z = self.min.z;
            next.y += 1;
        } else {
            self.next = None;
            return Some(pos);
        }
        self.next = Some(next);
        Some(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_walk_like_cuboid_and_count_what_is_left() {
        let (a, b) = (BlockPos::new(2, 5, -1), BlockPos::new(0, 3, 1));
        let mut area = Area::new(a, b);
        assert_eq!(area.remaining(), BlockPos::volume(a, b));
        let walked: Vec<BlockPos> = area.by_ref().take(10).collect();
        let expected: Vec<BlockPos> = BlockPos::cuboid(a, b).collect();
        assert_eq!(walked, expected[..10]);
        assert_eq!(area.remaining(), BlockPos::volume(a, b) - 10);
        assert_eq!(area.by_ref().count(), 17);
        assert_eq!(area.remaining(), 0);
    }

    #[test]
    fn snapshots_share_one_copy_of_each_state() {
        let snapshot: Snapshot = [
            (BlockPos::new(0, 0, 0), "stone"),
            (BlockPos::new(1, 0, 0), "stone"),
            (BlockPos::new(2, 0, 0), "dirt"),
            (BlockPos::new(3, 0, 0), "stone"),
        ]
        .into_iter()
        .collect();
        assert_eq!(snapshot.len(), 4);
        assert_eq!(snapshot.palette.len(), 2);
        let states: Vec<&str> = snapshot.iter().map(|(_, s)| s.as_str()).collect();
        assert_eq!(
            states,
            [
                "minecraft:stone",
                "minecraft:stone",
                "minecraft:dirt",
                "minecraft:stone"
            ]
        );
    }

    #[test]
    fn replace_matches_ids_or_whole_states() {
        let log = BlockState::from("oak_log[axis=y]");
        assert!(matches(&log, &BlockState::from("oak_log"), false));
        assert!(!matches(&log, &BlockState::from("oak"), false));
        assert!(matches(&log, &BlockState::from("oak_log[axis=y]"), true));
        assert!(!matches(&log, &BlockState::from("oak_log[axis=x]"), true));
        assert!(matches(
            &BlockState::from("stone"),
            &BlockState::from("stone"),
            false
        ));
    }

    #[test]
    fn progress_rounds_down() {
        assert_eq!(Progress { done: 1, total: 3 }.percent(), 33);
        assert_eq!(Progress { done: 0, total: 0 }.percent(), 100);
    }
}
