use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./../../client/src/js/bindings/")]
pub struct IapOffer {
    pub product_id: String,
    pub title: String,
    pub description: String,
    pub formatted_price: Option<String>,
}
