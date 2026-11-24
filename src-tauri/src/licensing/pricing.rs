//! Commercial seat price read from Polar's public organization storefront.
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::product_config::PRODUCT;

/// Price of one commercial seat in the currency's minor units.
#[derive(Clone, Debug, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SeatPrice {
    pub amount_minor: u32,
    /// Uppercase ISO 4217 code.
    pub currency: String,
}

#[derive(Deserialize)]
struct Storefront {
    products: Vec<Product>,
}

#[derive(Deserialize)]
struct Product {
    id: String,
    prices: Vec<Price>,
}

#[derive(Deserialize)]
struct Price {
    amount_type: String,
    price_currency: String,
    price_amount: Option<u32>,
    is_archived: bool,
    seat_tiers: Option<SeatTiers>,
}

#[derive(Deserialize)]
struct SeatTiers {
    tiers: Vec<SeatTier>,
}

#[derive(Deserialize)]
struct SeatTier {
    min_seats: u32,
    price_per_seat: u32,
}

/// Selects the single-seat price of the configured commercial product.
fn seat_price(bytes: &[u8]) -> Option<SeatPrice> {
    let storefront = serde_json::from_slice::<Storefront>(bytes).ok()?;
    let product = storefront
        .products
        .into_iter()
        .find(|product| product.id == PRODUCT.polar.product_id)?;
    let price = product
        .prices
        .into_iter()
        .find(|price| !price.is_archived)?;
    let amount_minor = match price.amount_type.as_str() {
        "seat_based" => {
            price
                .seat_tiers?
                .tiers
                .into_iter()
                .find(|tier| tier.min_seats <= 1)?
                .price_per_seat
        }
        "fixed" => price.price_amount?,
        _ => return None,
    };
    let currency = price.price_currency.to_ascii_uppercase();
    (currency.len() == 3 && currency.bytes().all(|byte| byte.is_ascii_alphabetic())).then_some(
        SeatPrice {
            amount_minor,
            currency,
        },
    )
}

/// Fetches the commercial seat price. Returns `None` when Polar is unreachable
/// or the product has no single-seat price; the UI then defers to checkout.
pub(super) async fn fetch_seat_price() -> Option<SeatPrice> {
    let response = super::polar::client()?
        .get(&PRODUCT.polar.storefront_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if bytes.len() > 1024 * 1024 {
        return None;
    }
    seat_price(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storefront(prices: serde_json::Value) -> Vec<u8> {
        serde_json::to_vec(&serde_json::json!({"products": [
            {"id": "other-product", "prices": [{"amount_type": "fixed", "price_currency": "usd", "price_amount": 1, "is_archived": false}]},
            {"id": PRODUCT.polar.product_id.as_str(), "prices": prices},
        ]}))
        .unwrap()
    }

    #[test]
    fn reads_the_single_seat_price_of_the_commercial_product() {
        let seat_based = storefront(serde_json::json!([
            {"amount_type": "fixed", "price_currency": "eur", "price_amount": 100, "is_archived": true},
            {"amount_type": "seat_based", "price_currency": "eur", "is_archived": false,
             "seat_tiers": {"tiers": [{"min_seats": 1, "max_seats": 9, "price_per_seat": 4900}, {"min_seats": 10, "price_per_seat": 3900}]}},
        ]));
        assert_eq!(
            seat_price(&seat_based),
            Some(SeatPrice {
                amount_minor: 4900,
                currency: "EUR".into()
            })
        );
        let fixed = storefront(serde_json::json!([
            {"amount_type": "fixed", "price_currency": "usd", "price_amount": 5950, "is_archived": false},
        ]));
        assert_eq!(
            seat_price(&fixed),
            Some(SeatPrice {
                amount_minor: 5950,
                currency: "USD".into()
            })
        );
        for unsupported in [
            storefront(
                serde_json::json!([{"amount_type": "custom", "price_currency": "eur", "is_archived": false}]),
            ),
            storefront(
                serde_json::json!([{"amount_type": "seat_based", "price_currency": "eur", "is_archived": false,
                "seat_tiers": {"tiers": [{"min_seats": 5, "price_per_seat": 3900}]}}]),
            ),
            b"<html>proxy failure</html>".to_vec(),
        ] {
            assert_eq!(seat_price(&unsupported), None);
        }
    }
}
