//! Strip credential-bearing query strings from URLs before logging or
//! publishing them. Some RPC providers put API keys in `?api_key=...`;
//! those must never reach log aggregators, `Debug` output, or public JSON.

/// Drop everything from `?` onward (query string and fragment-style tails).
///
/// The full URL with credentials is still stored on
/// [`crate::solana::SolanaClient`] for actual RPC/WebSocket calls; this
/// helper is only for safe surfaces (tracing, `Debug`, agent card).
pub fn public_origin(url: &str) -> String {
    match url.split_once('?') {
        Some((origin, _)) => origin.to_string(),
        None => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::public_origin;

    #[test]
    fn public_origin_strips_api_key_query() {
        assert_eq!(
            public_origin("https://ams.rpc.orbitflare.com?api_key=ORBIT-XXX"),
            "https://ams.rpc.orbitflare.com",
        );
    }

    #[test]
    fn public_origin_keeps_url_when_no_query() {
        assert_eq!(
            public_origin("https://api.devnet.solana.com"),
            "https://api.devnet.solana.com",
        );
    }

    #[test]
    fn public_origin_strips_only_first_question_mark() {
        assert_eq!(
            public_origin("https://rpc.example.com/foo?a=1?b=2"),
            "https://rpc.example.com/foo",
        );
    }
}
