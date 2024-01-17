use serde::{Deserialize, Serialize};

use crate::model::balloon::Balloon;

pub trait Json<'a> {
    type Item;
    // fn from_json(s: &'a String) -> serde_json::Result<Self::Item>
    // where
    //     <Self as Json<'a>>::Item: Deserialize<'a>,
    // {
    //     let b: Self::Item = serde_json::from_str(s.as_str())?;
    //     Ok(b)
    // }

    fn from_json(s: &'a str) -> serde_json::Result<Self::Item>
    where
        <Self as Json<'a>>::Item: Deserialize<'a>,
    {
        let b: Self::Item = serde_json::from_str(s)?;
        Ok(b)
    }

    fn to_json(&self) -> serde_json::Result<String>
    where
        Self: Serialize,
    {
        let s: String = serde_json::to_string(self)?;
        Ok(s)
    }

    fn into_json(self) -> serde_json::Result<String>
    where
        Self: Serialize + Sized,
    {
        let s: String = serde_json::to_string(&self)?;
        Ok(s)
    }
}

pub trait Metadata {
    fn from_raw_string(value: String) -> Result<Self, &'static str>
    where
        Self: Sized;
    fn to_raw_string(&self) -> Result<String, &'static str>;
}

impl Metadata for Balloon {
    fn from_raw_string(value: String) -> Result<Self, &'static str> {
        Balloon::from_json(value.as_str())
            .map_err(|_| "fail to deserialize json to Balloon")
    }

    fn to_raw_string(&self) -> Result<String, &'static str> {
        self.to_json().map_err(|_| "fail to serialize to a json style string")
    }
}
