use bvc_client_lib::StoreProvider;
use tauri_plugin_iap::{PricingPhase, Product, SubscriptionOffer};

fn phase(formatted_price: &str, price_amount_micros: i64) -> PricingPhase {
    PricingPhase {
        formatted_price: formatted_price.to_string(),
        price_currency_code: "USD".to_string(),
        price_amount_micros,
        billing_period: "P1M".to_string(),
        billing_cycle_count: 0,
        recurrence_mode: 1,
    }
}

fn product(
    formatted_price: Option<&str>,
    subscription_offer_details: Option<Vec<SubscriptionOffer>>,
) -> Product {
    Product {
        product_id: "realms.annual".to_string(),
        title: "Realms Connect".to_string(),
        description: "Proximity voice on every Realm.".to_string(),
        product_type: "subs".to_string(),
        formatted_price: formatted_price.map(str::to_string),
        price_currency_code: None,
        price_amount_micros: None,
        subscription_offer_details,
    }
}

// Android never sets the top-level price for subscriptions; it lives only under
// the offer's pricing phases. The price must come from the recurring (last)
// phase so the upsell tile is not blank.
#[test]
fn android_subscription_uses_nested_recurring_phase() {
    let offer = SubscriptionOffer {
        offer_token: "token".to_string(),
        base_plan_id: "annual".to_string(),
        offer_id: None,
        pricing_phases: vec![phase("Free", 0), phase("$14.99", 14_990_000)],
    };

    let price = StoreProvider::display_price(&product(None, Some(vec![offer])));

    assert_eq!(price, Some("$14.99".to_string()));
}

// iOS, macOS, and Windows report the price at the top level; it must win over
// any nested phase so those platforms are unaffected by the Android fallback.
#[test]
fn top_level_price_is_preferred_over_nested() {
    let offer = SubscriptionOffer {
        offer_token: "token".to_string(),
        base_plan_id: "annual".to_string(),
        offer_id: None,
        pricing_phases: vec![phase("$9.99", 9_990_000)],
    };

    let price = StoreProvider::display_price(&product(Some("$14.99"), Some(vec![offer])));

    assert_eq!(price, Some("$14.99".to_string()));
}

#[test]
fn missing_price_everywhere_yields_none() {
    assert_eq!(StoreProvider::display_price(&product(None, None)), None);
}
