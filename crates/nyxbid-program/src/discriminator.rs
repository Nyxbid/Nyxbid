//! 8-byte Anchor discriminators for every instruction, account, and event.
//!
//! Anchor computes these as the first 8 bytes of:
//!   - instruction: `sha256("global:<snake_case_name>")`
//!   - account:     `sha256("account:<PascalCaseName>")`
//!   - event:       `sha256("event:<PascalCaseName>")`
//!
//! The values below are extracted from `crates/nyxbid-program/idl/nyxbid.json`.
//! If any instruction/account/event is renamed in the chain crate, regenerate
//! the IDL and update this file.

/// Instruction discriminators (prefix bytes of the data field).
pub mod ix {
    pub const CANCEL: [u8; 8] = [232, 219, 223, 41, 219, 236, 220, 190];
    pub const CREATE_INTENT: [u8; 8] = [216, 214, 79, 121, 23, 194, 96, 104];
    pub const EXPIRE_NO_MAKER: [u8; 8] = [186, 90, 4, 217, 150, 158, 152, 29];
    pub const EXPIRE_WITH_MAKER: [u8; 8] = [117, 107, 110, 137, 83, 239, 240, 82];
    pub const FUND_MAKER_ESCROW: [u8; 8] = [118, 138, 216, 100, 167, 54, 223, 187];
    pub const REVEAL_QUOTE: [u8; 8] = [78, 23, 168, 150, 128, 0, 61, 134];
    pub const SETTLE: [u8; 8] = [175, 42, 185, 87, 144, 131, 102, 212];
    pub const SUBMIT_QUOTE: [u8; 8] = [230, 121, 122, 202, 228, 6, 91, 181];
}

/// Account discriminators (first 8 bytes of an account's data after init).
pub mod account {
    pub const ESCROW: [u8; 8] = [31, 213, 123, 187, 186, 22, 218, 155];
    pub const INTENT: [u8; 8] = [247, 162, 35, 165, 254, 111, 129, 109];
    pub const QUOTE: [u8; 8] = [167, 202, 20, 198, 228, 66, 105, 208];
    pub const RECEIPT: [u8; 8] = [39, 154, 73, 106, 80, 102, 145, 153];
    pub const REPUTATION: [u8; 8] = [55, 148, 90, 71, 68, 183, 193, 28];
}

/// Event discriminators (first 8 bytes of the base64-decoded `Program data:` payload).
pub mod event {
    pub const AUCTION_RESOLVED: [u8; 8] = [135, 86, 129, 72, 80, 120, 12, 248];
    pub const CANCELLED: [u8; 8] = [136, 23, 42, 65, 143, 233, 234, 46];
    pub const INTENT_CREATED: [u8; 8] = [184, 46, 156, 205, 169, 254, 11, 108];
    pub const QUOTE_REVEALED: [u8; 8] = [157, 221, 161, 57, 82, 192, 26, 200];
    pub const QUOTE_SUBMITTED: [u8; 8] = [207, 98, 251, 106, 249, 124, 126, 40];
    pub const SETTLED: [u8; 8] = [232, 210, 40, 17, 142, 124, 145, 238];
}
