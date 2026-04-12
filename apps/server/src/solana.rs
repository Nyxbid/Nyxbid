use borsh::BorshSerialize;
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::{Keypair, Signer};
use solana_pubkey::Pubkey;
use solana_signature::Signature;
use solana_transaction::Transaction;
use std::{str::FromStr, sync::Arc};

pub struct SolanaClient {
    rpc: RpcClient,
    delegate: Arc<Keypair>,
    program_id: Pubkey,
}

impl SolanaClient {
    pub fn from_env() -> Option<Self> {
        let rpc_url = std::env::var("SOLANA_RPC_URL").ok()?;
        let keypair_path = std::env::var("SOLANA_KEYPAIR_PATH").ok()?;
        let program_id_str = std::env::var("PAYQ_PROGRAM_ID").ok()?;

        let expanded = shellexpand::tilde(&keypair_path).to_string();
        let keypair_bytes: Vec<u8> = match std::fs::read_to_string(&expanded) {
            Ok(raw) => serde_json::from_str(&raw).ok()?,
            Err(_) => {
                tracing::warn!("could not read keypair from {expanded}, Solana integration disabled");
                return None;
            }
        };

        let secret: [u8; 32] = keypair_bytes
            .get(..32)?
            .try_into()
            .ok()?;
        let delegate = Keypair::new_from_array(secret);

        tracing::info!(
            rpc = %rpc_url,
            delegate = %delegate.pubkey(),
            program = %program_id_str,
            "solana client initialized"
        );

        Some(Self {
            rpc: RpcClient::new(rpc_url),
            delegate: Arc::new(delegate),
            program_id: Pubkey::from_str(&program_id_str).ok()?,
        })
    }

    pub fn delegate_pubkey(&self) -> Pubkey {
        self.delegate.pubkey()
    }

    pub async fn record_spend(
        &self,
        vault: Pubkey,
        agent_id: String,
        tool_id: String,
        amount: u64,
        proposal_hash: [u8; 32],
    ) -> Result<Signature, String> {
        let delegate = self.delegate.clone();
        let rpc_url = self.rpc.url();
        let program_id = self.program_id;

        tokio::task::spawn_blocking(move || {
            let rpc = RpcClient::new(rpc_url);

            let (spend_record_pda, _) = Pubkey::find_program_address(
                &[b"spend", vault.as_ref(), &proposal_hash],
                &program_id,
            );

            let system_program = solana_system_interface::program::id();

            let ix_data = build_record_spend_data(&agent_id, &tool_id, amount, proposal_hash)?;

            let ix = Instruction {
                program_id,
                accounts: vec![
                    AccountMeta::new(delegate.pubkey(), true),
                    AccountMeta::new(vault, false),
                    AccountMeta::new(spend_record_pda, false),
                    AccountMeta::new_readonly(system_program, false),
                ],
                data: ix_data,
            };

            let recent_blockhash = rpc
                .get_latest_blockhash()
                .map_err(|e| format!("get blockhash: {e}"))?;

            let tx = Transaction::new_signed_with_payer(
                &[ix],
                Some(&delegate.pubkey()),
                &[&delegate],
                recent_blockhash,
            );

            let sig = rpc
                .send_and_confirm_transaction(&tx)
                .map_err(|e| format!("send tx: {e}"))?;

            Ok(sig)
        })
        .await
        .map_err(|e| format!("join: {e}"))?
    }
}

fn record_spend_discriminator() -> [u8; 8] {
    let hash = Sha256::digest(b"global:record_spend");
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

fn build_record_spend_data(
    agent_id: &str,
    tool_id: &str,
    amount: u64,
    proposal_hash: [u8; 32],
) -> Result<Vec<u8>, String> {
    let mut data = Vec::new();
    data.extend_from_slice(&record_spend_discriminator());

    agent_id
        .to_string()
        .serialize(&mut data)
        .map_err(|e| format!("borsh agent_id: {e}"))?;
    tool_id
        .to_string()
        .serialize(&mut data)
        .map_err(|e| format!("borsh tool_id: {e}"))?;
    amount
        .serialize(&mut data)
        .map_err(|e| format!("borsh amount: {e}"))?;
    proposal_hash
        .serialize(&mut data)
        .map_err(|e| format!("borsh hash: {e}"))?;

    Ok(data)
}
