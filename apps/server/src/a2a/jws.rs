//! JWS-signed agent card support (A2A v1 §8.4).
//!
//! Workflow per the spec:
//!
//! 1. Serialise the unsigned card to JSON.
//! 2. Strip the `signatures` field if present.
//! 3. Canonicalise via RFC 8785 (JCS) — lexicographic key order,
//!    deterministic number formatting.
//! 4. Compute `b64url(protectedHeaderJSON) || "." || b64url(canonicalPayload)`.
//! 5. ES256-sign the concatenation, set `signature = b64url(rawSig)`.
//! 6. Embed an `AgentCardSignature { protected, signature }` back in
//!    the original (non-canonical) card.
//!
//! Verifiers reverse steps 2–5 against the public JWK published at
//! `/.well-known/jwks.json` (see [`derive_jwks`]).
//!
//! All of this is gated on the `A2A_SIGNING_KEY_PEM` environment
//! variable holding a PKCS#8 PEM-encoded ES256 private key. If the
//! variable is missing the card is served unsigned and `/jwks.json`
//! returns an empty key set.

use std::sync::OnceLock;

use base64::{
    engine::general_purpose::URL_SAFE_NO_PAD as B64URL,
    Engine as _,
};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use p256::{
    elliptic_curve::sec1::ToEncodedPoint,
    pkcs8::DecodePrivateKey,
    SecretKey,
};
use serde::Serialize;
use serde_json::{json, Value};

/// One A2A `AgentCardSignature` entry. The card carries a `signatures`
/// array; multiple signatures are allowed for key rotation.
#[derive(Clone, Debug, Serialize)]
pub struct AgentCardSignature {
    pub protected: String,
    pub signature: String,
    /// The spec also allows a `header` (unprotected) field; we omit it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Value>,
}

/// The signing material derived once at startup from
/// `A2A_SIGNING_KEY_PEM`. Holds both the encoding key (for signing)
/// and the public JWK (for `/jwks.json`).
pub struct SigningKey {
    encoding: EncodingKey,
    pub kid: String,
    pub public_jwk: Value,
}

static SIGNING_KEY: OnceLock<Option<SigningKey>> = OnceLock::new();

/// Initialise the signing key from `A2A_SIGNING_KEY_PEM` (a PKCS#8 PEM
/// blob). Returns `Ok(None)` when the env var is unset; that's the
/// "card served unsigned" path.
pub fn init() -> &'static Option<SigningKey> {
    SIGNING_KEY.get_or_init(|| match std::env::var("A2A_SIGNING_KEY_PEM") {
        Err(_) => None,
        Ok(pem) => match load_signing_key(&pem) {
            Ok(k) => {
                tracing::info!(kid = %k.kid, "A2A agent-card signing enabled");
                Some(k)
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "A2A_SIGNING_KEY_PEM is set but failed to parse — serving card unsigned"
                );
                None
            }
        },
    })
}

fn load_signing_key(pem: &str) -> Result<SigningKey, String> {
    // Parse the PKCS#8 PEM into a p256 SecretKey so we can derive the
    // public JWK. The same PEM is then handed to jsonwebtoken which
    // wants the same format.
    let secret = SecretKey::from_pkcs8_pem(pem).map_err(|e| format!("parse pkcs8: {e}"))?;
    let pub_point = secret.public_key().to_encoded_point(false);
    let x = pub_point
        .x()
        .ok_or_else(|| "missing X coordinate".to_string())?;
    let y = pub_point
        .y()
        .ok_or_else(|| "missing Y coordinate".to_string())?;
    let kid = format!("nyxbid-es256-{}", hex::encode(&x[..4]));
    let public_jwk = json!({
        "kty": "EC",
        "crv": "P-256",
        "use": "sig",
        "alg": "ES256",
        "kid": kid,
        "x": B64URL.encode(x),
        "y": B64URL.encode(y),
    });
    let encoding =
        EncodingKey::from_ec_pem(pem.as_bytes()).map_err(|e| format!("encoding key: {e}"))?;
    Ok(SigningKey {
        encoding,
        kid,
        public_jwk,
    })
}

/// JSON Web Key Set served at `/.well-known/jwks.json`. Empty when the
/// agent runs unsigned.
pub fn derive_jwks() -> Value {
    let keys: Vec<Value> = init()
        .as_ref()
        .map(|k| vec![k.public_jwk.clone()])
        .unwrap_or_default();
    json!({ "keys": keys })
}

/// Take an unsigned card serialized as a `serde_json::Value` and
/// return the same value with a populated `signatures` array. If
/// signing is disabled the input is returned untouched.
pub fn maybe_sign(mut card: Value) -> Value {
    let Some(key) = init().as_ref() else {
        return card;
    };

    // 1. Strip any pre-existing signatures field, then JCS-canonicalise.
    if let Some(obj) = card.as_object_mut() {
        obj.remove("signatures");
    }
    let canonical = match serde_jcs::to_string(&card) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "JCS canonicalization failed; serving card unsigned");
            return card;
        }
    };

    // 2. Build a signing-only ES256 JWS over `b64url(header) . b64url(payload)`.
    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some(key.kid.clone());
    header.typ = Some("JOSE".to_string());
    let header_b64 = match serde_json::to_vec(&header) {
        Ok(v) => B64URL.encode(v),
        Err(_) => return card,
    };
    let payload_b64 = B64URL.encode(canonical.as_bytes());
    let signing_input = format!("{header_b64}.{payload_b64}");

    let signature_b64 = match jsonwebtoken::crypto::sign(
        signing_input.as_bytes(),
        &key.encoding,
        Algorithm::ES256,
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "ES256 signature failed; serving card unsigned");
            return card;
        }
    };

    let entry = AgentCardSignature {
        protected: header_b64,
        signature: signature_b64,
        header: None,
    };

    if let Some(obj) = card.as_object_mut() {
        obj.insert(
            "signatures".to_string(),
            serde_json::to_value(vec![entry]).unwrap_or(Value::Null),
        );
    }
    card
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::pkcs8::EncodePrivateKey;
    use p256::SecretKey;
    use rand_core::OsRng;

    /// Without env set, signing is a no-op and the JWKS is empty.
    #[test]
    fn unsigned_when_env_missing() {
        // SIGNING_KEY may have been initialized in a previous test run.
        // We can't reliably reset OnceLock, so this test only asserts
        // that *if* unset, derive_jwks returns an empty list.
        if std::env::var("A2A_SIGNING_KEY_PEM").is_err() {
            let jwks = derive_jwks();
            assert_eq!(jwks["keys"].as_array().unwrap().len(), 0);
        }
    }

    /// The protected header must be a base64url-encoded JSON object
    /// declaring `alg: ES256` per A2A §8.4.2.
    #[test]
    fn protected_header_parses_as_jose() {
        let header = Header::new(Algorithm::ES256);
        let header_bytes = serde_json::to_vec(&header).unwrap();
        let header_b64 = B64URL.encode(&header_bytes);
        let decoded = B64URL.decode(header_b64.as_bytes()).unwrap();
        let parsed: Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(parsed["alg"], "ES256");
    }

    /// End-to-end: generate an ES256 keypair, run [`load_signing_key`]
    /// on its PEM, and verify the resulting public JWK contains the
    /// canonical EC fields (`kty`, `crv`, `x`, `y`, `kid`).
    #[test]
    fn load_signing_key_round_trips_pkcs8_pem() {
        let secret = SecretKey::random(&mut OsRng);
        let pem = secret.to_pkcs8_pem(Default::default()).unwrap();
        let key = load_signing_key(&pem).expect("load signing key");
        assert_eq!(key.public_jwk["kty"], "EC");
        assert_eq!(key.public_jwk["crv"], "P-256");
        assert_eq!(key.public_jwk["alg"], "ES256");
        assert!(key.public_jwk["x"].as_str().is_some());
        assert!(key.public_jwk["y"].as_str().is_some());
        assert!(key.kid.starts_with("nyxbid-es256-"));
    }
}
