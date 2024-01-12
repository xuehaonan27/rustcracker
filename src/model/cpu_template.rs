use serde::{Serialize, Deserialize};

use crate::utils::Json;

// pub struct CPUTemplate {}
pub type CPUTemplate = String;

pub const CPU_TEMPLATE_C3: &'static str = "C3";
pub const CPU_TEMPLATE_T2: &'static str = "T2";

#[derive(Serialize, Deserialize)]
pub struct CPUConfig {}

impl<'a> Json<'a> for CPUConfig {
    type Item = CPUConfig;
}