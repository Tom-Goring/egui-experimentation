use serde::Serialize;

#[allow(dead_code)]
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Command {
    ListParameters,
    ListSignals,
    GetParameterValue { name: String },
    SetParameterValue { name: String, value: f64 },
    SubscribeToSignal { name: String },
}