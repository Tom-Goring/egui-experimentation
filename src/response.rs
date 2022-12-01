use std::collections::HashMap;

use serde::{ Serialize, Deserialize };

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Parameters(HashMap<String, f64>),
    Done
}