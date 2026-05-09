//! On-chain wire types for the `nyxbid` Anchor program.
//!
//! This crate is the source of truth for everything the off-chain server
//! and SDKs need to talk to the program **without** depending on
//! `anchor-lang` or pulling in the chain crate. It contains:
//!
//! - The program ID and well-known SPL/system program IDs.
//! - PDA seed bytes and derivation helpers.
//! - Borsh-serialisable instruction parameter structs.
//! - Borsh-deserialisable account state and event structs.
//! - 8-byte Anchor discriminators for every instruction, account, and
//!   event in the program.
//! - The full IDL bundled at compile time via [`IDL_JSON`].
//!
//! The wire format is locked to whatever `chain/programs/nyxbid` last
//! emitted; run `just sync-idl` after any program change to refresh
//! [`IDL_JSON`] alongside the discriminator constants.

pub mod discriminator;
pub mod error;
pub mod events;
pub mod id;
pub mod params;
pub mod pda;
pub mod seeds;
pub mod state;

/// Raw IDL JSON, bundled at compile time from `crates/nyxbid-program/idl/nyxbid.json`.
///
/// Useful for clients that want to introspect instructions or surface
/// type information without re-parsing the chain folder.
pub const IDL_JSON: &str = include_str!("../idl/nyxbid.json");

pub use error::{DecodeError, NyxbidError};

/// Strip and validate an 8-byte Anchor discriminator, then borsh-decode
/// the remainder into `T`. Shared by [`AnchorAccount`] and [`AnchorEvent`].
fn decode_with_discriminator<T: borsh::BorshDeserialize>(
    data: &[u8],
    expected: [u8; 8],
) -> Result<T, DecodeError> {
    if data.len() < 8 {
        return Err(DecodeError::TooShort {
            got: data.len(),
            want: 8,
        });
    }
    let (disc, body) = data.split_at(8);
    if disc != expected {
        return Err(DecodeError::WrongDiscriminator);
    }
    T::try_from_slice(body).map_err(DecodeError::Borsh)
}

/// Convenience trait for Anchor-style accounts: `[8-byte discriminator] || borsh(fields)`.
pub trait AnchorAccount: borsh::BorshDeserialize {
    /// 8-byte discriminator written by Anchor at account init.
    const DISCRIMINATOR: [u8; 8];

    /// Decode an account from its raw on-chain data, validating the
    /// 8-byte discriminator prefix.
    fn try_decode(data: &[u8]) -> Result<Self, DecodeError> {
        decode_with_discriminator::<Self>(data, Self::DISCRIMINATOR)
    }
}

/// Convenience trait for Anchor-style events emitted via `emit!`.
///
/// Anchor encodes events as `[8-byte discriminator] || borsh(fields)`,
/// then base64-encodes that blob and prefixes the program log line with
/// `Program data: ` (note: not `Program log:`).
pub trait AnchorEvent: borsh::BorshDeserialize {
    /// 8-byte discriminator computed from `event:<EventName>`.
    const DISCRIMINATOR: [u8; 8];

    /// Decode the raw bytes after the `Program data:` log prefix has
    /// been base64-decoded.
    fn try_decode(data: &[u8]) -> Result<Self, DecodeError> {
        decode_with_discriminator::<Self>(data, Self::DISCRIMINATOR)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cross-check every constant against the bundled IDL so a chain
    /// rename + `just sync-idl` will fail compilation here loudly.
    #[test]
    fn discriminators_match_idl() {
        let idl: serde_json::Value =
            serde_json::from_str(IDL_JSON).expect("idl json parses");
        let to_bytes = |v: &serde_json::Value| -> [u8; 8] {
            let arr = v.as_array().expect("discriminator is array");
            assert_eq!(arr.len(), 8);
            let mut out = [0u8; 8];
            for (i, b) in arr.iter().enumerate() {
                out[i] = b.as_u64().unwrap() as u8;
            }
            out
        };
        let lookup = |bucket: &str, name: &str| -> [u8; 8] {
            let arr = idl[bucket].as_array().expect("bucket exists");
            for entry in arr {
                if entry["name"] == name {
                    return to_bytes(&entry["discriminator"]);
                }
            }
            panic!("{name} not in {bucket}");
        };

        assert_eq!(discriminator::ix::CANCEL, lookup("instructions", "cancel"));
        assert_eq!(
            discriminator::ix::CREATE_INTENT,
            lookup("instructions", "create_intent")
        );
        assert_eq!(
            discriminator::ix::SUBMIT_QUOTE,
            lookup("instructions", "submit_quote")
        );
        assert_eq!(
            discriminator::ix::REVEAL_QUOTE,
            lookup("instructions", "reveal_quote")
        );
        assert_eq!(
            discriminator::ix::FUND_MAKER_ESCROW,
            lookup("instructions", "fund_maker_escrow")
        );
        assert_eq!(discriminator::ix::SETTLE, lookup("instructions", "settle"));
        assert_eq!(
            discriminator::ix::EXPIRE_WITH_MAKER,
            lookup("instructions", "expire_with_maker")
        );
        assert_eq!(
            discriminator::ix::EXPIRE_NO_MAKER,
            lookup("instructions", "expire_no_maker")
        );

        assert_eq!(discriminator::account::INTENT, lookup("accounts", "Intent"));
        assert_eq!(discriminator::account::QUOTE, lookup("accounts", "Quote"));
        assert_eq!(discriminator::account::ESCROW, lookup("accounts", "Escrow"));
        assert_eq!(
            discriminator::account::RECEIPT,
            lookup("accounts", "Receipt")
        );
        assert_eq!(
            discriminator::account::REPUTATION,
            lookup("accounts", "Reputation")
        );

        assert_eq!(
            discriminator::event::INTENT_CREATED,
            lookup("events", "IntentCreated")
        );
        assert_eq!(
            discriminator::event::QUOTE_SUBMITTED,
            lookup("events", "QuoteSubmitted")
        );
        assert_eq!(
            discriminator::event::QUOTE_REVEALED,
            lookup("events", "QuoteRevealed")
        );
        assert_eq!(
            discriminator::event::AUCTION_RESOLVED,
            lookup("events", "AuctionResolved")
        );
        assert_eq!(
            discriminator::event::SETTLED,
            lookup("events", "Settled")
        );
        assert_eq!(
            discriminator::event::CANCELLED,
            lookup("events", "Cancelled")
        );
    }

    /// `Intent` PDA depends on the on-chain seed being exactly `b"intent"`.
    /// If anyone changes that constant in chain/state.rs, this will catch it.
    #[test]
    fn pda_seeds_match_chain_constants() {
        // Pure structural check: just verify the byte values.
        assert_eq!(seeds::INTENT, b"intent");
        assert_eq!(seeds::QUOTE, b"quote");
        assert_eq!(seeds::ESCROW, b"escrow");
        assert_eq!(seeds::TAKER_VAULT, b"taker_vault");
        assert_eq!(seeds::MAKER_VAULT, b"maker_vault");
        assert_eq!(seeds::RECEIPT, b"receipt");
        assert_eq!(seeds::REPUTATION, b"reputation");
    }

    #[test]
    fn empty_ix_data_is_just_discriminator() {
        let data = params::encode_empty_ix_data(discriminator::ix::SETTLE);
        assert_eq!(data, discriminator::ix::SETTLE.to_vec());
    }

    #[test]
    fn errors_round_trip_through_codes() {
        for code in 6000u32..=6028 {
            let e = NyxbidError::from_code(code).expect("known code");
            assert_eq!(e.code(), code);
        }
        assert!(NyxbidError::from_code(5999).is_none());
        assert!(NyxbidError::from_code(6029).is_none());
    }
}
