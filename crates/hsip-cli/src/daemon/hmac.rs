// HMAC Response Integrity Protection
// Protects against OWASP A08 (Software and Data Integrity Failures)

use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::Serialize;

type HmacSha256 = Hmac<Sha256>;

// Production key should be stored securely, not hardcoded
const RESPONSE_HMAC_KEY: &[u8] = b"HSIP-DAEMON-RESPONSE-INTEGRITY-KEY-V1-CHANGE-IN-PRODUCTION";

#[derive(Serialize)]
pub struct SignedResponse<T: Serialize> {
    pub data: T,
    pub signature: String,
    #[serde(rename = "sig_alg")]
    pub signature_algorithm: String,
}

pub fn sign_response<T: Serialize>(data: &T) -> Result<String, String> {
    let json_bytes = serde_json::to_vec(data).map_err(|e| e.to_string())?;
    let mut mac = HmacSha256::new_from_slice(RESPONSE_HMAC_KEY)
        .map_err(|e| e.to_string())?;
    mac.update(&json_bytes);
    let signature = mac.finalize().into_bytes();
    Ok(hex::encode(signature))
}

pub fn create_signed_json<T: Serialize>(data: T) -> Result<String, String> {
    let signature = sign_response(&data)?;
    let signed = SignedResponse {
        data,
        signature,
        signature_algorithm: "HMAC-SHA256".to_string(),
    };
    serde_json::to_string(&signed).map_err(|e| e.to_string())
}
