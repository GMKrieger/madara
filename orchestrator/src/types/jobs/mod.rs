pub mod external_id;
pub mod job_item;
pub mod job_updates;
pub mod metadata;
pub mod status;
pub mod types;

use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;
use strum_macros::Display;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Display, strum_macros::EnumString)]
#[strum(serialize_all = "PascalCase")]
pub enum WorkerTriggerType {
    Snos,
    Proving,
    ProofRegistration,
    DataSubmission,
    UpdateState,
    Batching,
}

#[derive(Debug, Serialize, Clone)]
pub struct WorkerTriggerMessage {
    pub worker: WorkerTriggerType,
}

#[derive(Error, Debug)]
pub enum WorkerTriggerTypeError {
    #[error("Unknown WorkerTriggerType: {0}")]
    UnknownType(String),
}

// TODO : Need to check why serde deserializer was failing here.
// TODO : Remove this custom deserializer.
/// Implemented a custom deserializer as when using serde json deserializer
/// It was unable to deserialize the response from the event trigger.
impl<'de> Deserialize<'de> for WorkerTriggerMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        struct Helper {
            worker: String,
        }
        let helper = Helper::deserialize(deserializer)?;
        Ok(WorkerTriggerMessage {
            worker: WorkerTriggerType::from_str(&helper.worker).map_err(serde::de::Error::custom)?,
        })
    }
}
