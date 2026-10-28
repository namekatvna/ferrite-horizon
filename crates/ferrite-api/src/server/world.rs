use crate::game::{BlockPos, Dimension};
use crate::server::BlockState;
use crate::{Error, host};

pub const MAX_CHANGES_PER_CALL: usize = 4096;

pub fn block(dimension: &Dimension, pos: BlockPos) -> Option<BlockState> {
    host::server_get_block(dimension.id(), pos).map(BlockState::new)
}

pub fn biome(dimension: &Dimension, pos: BlockPos) -> Option<String> {
    host::server_get_biome(dimension.id(), pos)
}

pub fn set_block(
    dimension: &Dimension,
    pos: BlockPos,
    state: impl Into<BlockState>,
) -> Result<(), Error> {
    set_blocks(dimension, [(pos, state)]).map(|_| ())
}

pub fn set_blocks<I, S>(dimension: &Dimension, changes: I) -> Result<usize, Error>
where
    I: IntoIterator<Item = (BlockPos, S)>,
    S: Into<BlockState>,
{
    let mut batch: Vec<(BlockPos, String)> = Vec::new();
    let mut total = 0;
    for (pos, state) in changes {
        let state: BlockState = state.into();
        batch.push((pos, state.into()));
        if batch.len() == MAX_CHANGES_PER_CALL {
            total += send(dimension, &mut batch)?;
        }
    }
    if !batch.is_empty() {
        total += send(dimension, &mut batch)?;
    }
    Ok(total)
}

fn send(dimension: &Dimension, batch: &mut Vec<(BlockPos, String)>) -> Result<usize, Error> {
    host::server_set_blocks(dimension.id(), batch).map_err(Error::new)?;
    let sent = batch.len();
    batch.clear();
    Ok(sent)
}
