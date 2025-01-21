use std::{ops::Range, sync::Arc};

use mc_db::MadaraBackend;
use mp_state_update::StateDiff;

use crate::{
    pipeline::{ApplyOutcome, PipelineController, PipelineSteps},
    import::BlockImporter,
};

pub type ApplyStateSync = PipelineController<ApplyStateSteps>;
pub fn apply_state_pipeline(
    backend: Arc<MadaraBackend>,
    importer: Arc<BlockImporter>,
    parallelization: usize,
    batch_size: usize,
) -> ApplyStateSync {
    PipelineController::new(ApplyStateSteps { backend, importer }, parallelization, batch_size)
}
pub struct ApplyStateSteps {
    backend: Arc<MadaraBackend>,
    importer: Arc<BlockImporter>,
}

impl PipelineSteps for ApplyStateSteps {
    type InputItem = StateDiff;
    type SequentialStepInput = Vec<StateDiff>;
    type Output = ();

    async fn parallel_step(
        self: Arc<Self>,
        _block_range: Range<u64>,
        input: Vec<Self::InputItem>,
    ) -> anyhow::Result<Self::SequentialStepInput> {
        Ok(input)
    }

    async fn sequential_step(
        self: Arc<Self>,
        block_range: Range<u64>,
        input: Self::SequentialStepInput,
    ) -> anyhow::Result<ApplyOutcome<Self::Output>> {
        tracing::debug!("Apply state sequential step {block_range:?}");
        self.importer.apply_to_global_trie(block_range, input).await?;
        Ok(ApplyOutcome::Success(()))
    }

    fn starting_block_n(&self) -> Option<u64> {
        self.backend.head_status().global_trie.get()
    }
}
