use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../client/src/models/generated/")]
pub struct StoreChain {
    pub id: String,
    pub name: String,
    pub uses_flipp: bool,
}

pub fn supported_chains() -> Vec<StoreChain> {
    vec![
        StoreChain {
            id: "qfc".into(),
            name: "QFC".into(),
            uses_flipp: true,
        },
        StoreChain {
            id: "safeway".into(),
            name: "Safeway".into(),
            uses_flipp: true,
        },
        StoreChain {
            id: "fred-meyer".into(),
            name: "Fred Meyer".into(),
            uses_flipp: true,
        },
        StoreChain {
            id: "whole-foods".into(),
            name: "Whole Foods".into(),
            uses_flipp: false,
        },
    ]
}
