use std::borrow::Cow;

use crate::game::{BlockPos, Dimension};
use crate::server::BlockState;
use crate::{Error, host};

pub use crate::server::edit::{
    DEFAULT_EDITS_PER_TICK, Edit, EditDone, EditId, MAX_EDITS_PER_TICK, Progress, Snapshot, cancel,
    pending_edits, progress,
};

pub const MAX_CHANGES_PER_CALL: usize = 4096;

pub fn block(dimension: &Dimension, pos: BlockPos) -> Option<BlockState> {
    host::server_get_block(dimension.id(), pos).map(BlockState::from_host)
}

pub fn blocks(
    dimension: &Dimension,
    a: BlockPos,
    b: BlockPos,
) -> impl Iterator<Item = (BlockPos, BlockState)> {
    let dimension = dimension.id().to_owned();
    BlockPos::cuboid(a, b).filter_map(move |pos| {
        host::server_get_block(&dimension, pos).map(|state| (pos, BlockState::from_host(state)))
    })
}

pub fn biome(dimension: &Dimension, pos: BlockPos) -> Option<String> {
    host::server_get_biome(dimension.id(), pos)
}

pub trait AsBlockState<'a> {
    #[doc(hidden)]
    fn block_state(self) -> Cow<'a, str>;
}

impl<'a> AsBlockState<'a> for BlockState {
    fn block_state(self) -> Cow<'a, str> {
        Cow::Owned(self.0)
    }
}

impl<'a> AsBlockState<'a> for &'a BlockState {
    fn block_state(self) -> Cow<'a, str> {
        Cow::Borrowed(self.as_str())
    }
}

impl<'a> AsBlockState<'a> for &'a str {
    fn block_state(self) -> Cow<'a, str> {
        BlockState::text(self)
    }
}

impl<'a> AsBlockState<'a> for &'a String {
    fn block_state(self) -> Cow<'a, str> {
        BlockState::text(self)
    }
}

impl<'a> AsBlockState<'a> for String {
    fn block_state(self) -> Cow<'a, str> {
        Cow::Owned(BlockState::new(self).0)
    }
}

pub fn set_block<'a>(
    dimension: &Dimension,
    pos: BlockPos,
    state: impl AsBlockState<'a>,
) -> Result<(), Error> {
    set_blocks(dimension, [(pos, state)]).map(|_| ())
}

pub fn set_blocks<'a, I, S>(dimension: &Dimension, changes: I) -> Result<usize, Error>
where
    I: IntoIterator<Item = (BlockPos, S)>,
    S: AsBlockState<'a>,
{
    let mut changes = changes.into_iter();
    let mut batch: Vec<(BlockPos, Cow<'a, str>)> =
        Vec::with_capacity(changes.size_hint().0.min(MAX_CHANGES_PER_CALL));
    let mut total = 0;
    loop {
        batch.clear();
        batch.extend(
            changes
                .by_ref()
                .take(MAX_CHANGES_PER_CALL)
                .map(|(pos, state)| (pos, state.block_state())),
        );
        if batch.is_empty() {
            return Ok(total);
        }
        total += send(
            dimension,
            batch.iter().map(|(pos, state)| (*pos, state.as_ref())),
        )?;
    }
}

pub fn fill<'a>(
    dimension: &Dimension,
    a: BlockPos,
    b: BlockPos,
    state: impl AsBlockState<'a>,
) -> Result<usize, Error> {
    let state = state.block_state();
    let mut positions = BlockPos::cuboid(a, b);
    let mut total = 0;
    loop {
        let sent = send(
            dimension,
            positions
                .by_ref()
                .take(MAX_CHANGES_PER_CALL)
                .map(|pos| (pos, state.as_ref())),
        )?;
        if sent == 0 {
            return Ok(total);
        }
        total += sent;
    }
}

fn send<'a>(
    dimension: &Dimension,
    changes: impl Iterator<Item = (BlockPos, &'a str)>,
) -> Result<usize, Error> {
    let changes: Vec<(BlockPos, &str)> = changes.collect();
    if changes.is_empty() {
        return Ok(0);
    }
    host::server_set_blocks(dimension.id(), &changes).map_err(Error::new)?;
    Ok(changes.len())
}
