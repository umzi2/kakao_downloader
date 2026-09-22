use chrono::Utc;

use crate::kakao::chapters::response::{ ItemIndex, PurchaseInfo };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purchase {
    NotPurchased,
    Rent,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    pub index: usize,
    pub id: usize,
    pub purchase: Purchase,
    pub rentable: bool,
    pub title: String,
}

impl Chapter {
    pub fn needed_rental(&self) -> bool {
        self.rentable && self.purchase != Purchase::Rent
    }
}

impl From<ItemIndex> for Chapter {
    fn from(item: ItemIndex) -> Self {
        let purchase = match item.item.service_property.purchase_info {
            None => Purchase::Unknown,
            Some(value) =>
                match value {
                    PurchaseInfo::NotPurchased => Purchase::NotPurchased,
                    PurchaseInfo::Rent { rent_expire_dt } => {
                        if rent_expire_dt < Utc::now() {
                            Purchase::NotPurchased
                        } else {
                            Purchase::Rent
                        }
                    }
                }
        };
        Self {
            index: item.item.order_value,
            id: item.item.product_id,
            purchase,
            rentable: !item.item.waitfree_blocked,
            title: item.item.title,
        }
    }
}
