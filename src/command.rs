use std::sync::Arc;

use serde::Serialize;

#[allow(dead_code)]
#[derive(Serialize, Debug, Clone)]
#[serde(tag="type")]
pub enum Command {
    ListParameters,
    ListSignals,
    GetParameterValue { name: Arc<String> },
    SetParameterValue { name: String, value: f64 },
    SubscribeToSignal { name: String }
}
