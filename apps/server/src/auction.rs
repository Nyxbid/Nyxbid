use nyxbid_types::{Intent, Quote, Side};

pub fn pick_winner<'a>(intent: &Intent, quotes: &'a [Quote]) -> Option<&'a Quote> {
    let revealed: Vec<&Quote> = quotes.iter().filter(|q| q.revealed).collect();
    if revealed.is_empty() {
        return None;
    }

    match intent.side {
        Side::Buy => revealed
            .into_iter()
            .min_by_key(|q| q.revealed_price.unwrap_or(u64::MAX)),
        Side::Sell => revealed
            .into_iter()
            .max_by_key(|q| q.revealed_price.unwrap_or(0)),
    }
}

pub fn clears_limit(intent: &Intent, price: u64) -> bool {
    match intent.side {
        Side::Buy => price <= intent.limit_price,
        Side::Sell => price >= intent.limit_price,
    }
}
