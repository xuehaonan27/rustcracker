use serde::{Serialize, Deserialize};

use crate::utils::Json;

/// The CPU Template defines a set of flags to be disabled from the microvm so that
/// the features exposed to the guest are the same as in the selected instance type.
/// This parameter has been deprecated and it will be removed in future Firecracker
/// release.
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CPUTemplate(
    /// default: "None"
    pub CPUTemplateString
);

#[derive(Serialize, Deserialize, Debug, Clone,Copy)]
pub enum CPUTemplateString {
    #[serde(rename = "C3")]
    C3,
    #[serde(rename = "T2")]
    T2,
    #[serde(rename = "T2S")]
    T2S,
    #[serde(rename = "T2CL")]
    T2CL,
    #[serde(rename = "T2A")]
    T2A,
    #[serde(rename = "V1N1")]
    V1N1,
    #[serde(rename = "None")]
    None,
}

/// The CPU configuration template defines a set of bit maps as modifiers 
/// of flags accessed by register to be disabled/enabled for the microvm.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CPUConfig();

impl<'a> Json<'a> for CPUConfig {
    type Item = CPUConfig;
}

impl<'a> Json<'a> for CPUTemplate {
    type Item = CPUTemplate;
}