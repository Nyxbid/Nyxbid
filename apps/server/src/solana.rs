use std::env;

#[derive(Debug, Clone)]
pub struct SolanaClient {
    pub rpc_url: String,
    pub program_id: String,
    pub usdc_mint: String,
}

impl SolanaClient {
    pub fn from_env() -> Option<Self> {
        let rpc_url = env::var("SOLANA_RPC_URL").ok()?;
        let program_id = env::var("NYXBID_PROGRAM_ID").ok()?;
        let usdc_mint = env::var("NYXBID_USDC_MINT")
            .unwrap_or_else(|_| "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU".to_string());

        Some(Self {
            rpc_url,
            program_id,
            usdc_mint,
        })
    }
}
