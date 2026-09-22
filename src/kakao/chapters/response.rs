use chrono::{ DateTime, FixedOffset };
use serde::{ Deserialize, Serialize };

#[derive(Debug, Deserialize)]
pub struct ChapterList {
    pub result: ChapterResult,
}

#[derive(Debug, Deserialize)]
pub struct ChapterResult {
    pub total_count: usize,
    pub list: Vec<ItemIndex>,
}

#[derive(Debug, Deserialize)]
pub struct ItemIndex {
    pub item: Item,
}

#[derive(Debug, Deserialize)]
pub struct Item {
    pub service_property: ServiceProperty,
    pub waitfree_blocked: bool,
    pub product_id: usize,
    pub title: String,
    pub order_value: usize,
}

#[derive(Debug, Deserialize)]
pub struct ServiceProperty {
    pub purchase_info: Option<PurchaseInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "purchase_type", rename_all = "snake_case")]
pub enum PurchaseInfo {
    NotPurchased,
    Rent {
        rent_expire_dt: DateTime<FixedOffset>,
    },
}
