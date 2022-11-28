use serde::Serialize;
use serde_json::Number;

#[allow(dead_code)]
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Command {
    ListParameters,
    ListSignals,
    GetParameterValue { name: String },
    SetParameterValue { name: String, value: Number },
    SubscribeToSignal { name: String },
    CloseListenerThread
}