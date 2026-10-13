//! JSON Web Token decoder, with expiry checks and HMAC (HS256/384/512) signature verification.

use crate::encoding::decode_base64;
use hmac::{Hmac, KeyInit, Mac};
use jiff::Timestamp;
use serde_json::Value;
use sha2::{Sha256, Sha384, Sha512};
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct Jwt {
    pub header: Value,
    pub payload: Value,
    pub signature: Vec<u8>,
    /// `<header>.<payload>` as it appeared in the token: the bytes the signature covers.
    signing_input: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    /// Five parts: an encrypted JWE, which can't be read without the key.
    Encrypted,
    WrongPartCount(usize),
    InvalidBase64(&'static str),
    InvalidJson(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "Paste a JWT"),
            Error::Encrypted => write!(
                f,
                "This is an encrypted token (JWE). Its contents can't be read without the key"
            ),
            Error::WrongPartCount(count) => {
                write!(
                    f,
                    "A JWT has 3 parts separated by dots (header.payload.signature); this has {count}"
                )
            }
            Error::InvalidBase64(part) => write!(f, "The {part} isn't valid Base64URL"),
            Error::InvalidJson(part) => write!(f, "The {part} isn't a JSON object"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Expired { at: Timestamp },
    NotYetValid { from: Timestamp },
    Valid { until: Option<Timestamp> },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verification {
    Valid,
    Invalid,
    /// Only HMAC algorithms can be checked with a shared secret.
    Unsupported(String),
}

/// Decodes a JWT, ignoring surrounding whitespace and a `Bearer ` prefix.
pub fn decode(token: &str) -> Result<Jwt, Error> {
    let token = token.trim();
    let token = token.strip_prefix("Bearer ").unwrap_or(token).trim();
    if token.is_empty() {
        return Err(Error::Empty);
    }
    let parts: Vec<&str> = token.split('.').collect();
    match parts.len() {
        3 => {}
        5 => return Err(Error::Encrypted),
        count => return Err(Error::WrongPartCount(count)),
    }
    let json = |part: &str, name: &'static str| -> Result<Value, Error> {
        let bytes = decode_base64(part).map_err(|_| Error::InvalidBase64(name))?;
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(value @ Value::Object(_)) => Ok(value),
            _ => Err(Error::InvalidJson(name)),
        }
    };
    Ok(Jwt {
        header: json(parts[0], "header")?,
        payload: json(parts[1], "payload")?,
        signature: decode_base64(parts[2]).map_err(|_| Error::InvalidBase64("signature"))?,
        signing_input: format!("{}.{}", parts[0], parts[1]),
    })
}

impl Jwt {
    pub fn algorithm(&self) -> Option<&str> {
        self.header.get("alg").and_then(Value::as_str)
    }

    pub fn header_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.header).unwrap_or_default()
    }

    pub fn payload_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.payload).unwrap_or_default()
    }

    /// A NumericDate claim (seconds since the Unix epoch) such as `exp`, `nbf` or `iat`.
    pub fn time_claim(&self, name: &str) -> Option<Timestamp> {
        let seconds = self.payload.get(name)?.as_f64()?;
        Timestamp::from_millisecond((seconds * 1000.0).round() as i64).ok()
    }

    pub fn status(&self, now: Timestamp) -> Status {
        let expires = self.time_claim("exp");
        if let Some(at) = expires.filter(|exp| now >= *exp) {
            return Status::Expired { at };
        }
        if let Some(from) = self.time_claim("nbf").filter(|nbf| now < *nbf) {
            return Status::NotYetValid { from };
        }
        Status::Valid { until: expires }
    }

    /// Checks an HMAC signature against `secret`, in constant time.
    pub fn verify(&self, secret: &[u8]) -> Verification {
        let input = self.signing_input.as_bytes();
        let valid = match self.algorithm() {
            Some("HS256") => verify_hmac::<Hmac<Sha256>>(secret, input, &self.signature),
            Some("HS384") => verify_hmac::<Hmac<Sha384>>(secret, input, &self.signature),
            Some("HS512") => verify_hmac::<Hmac<Sha512>>(secret, input, &self.signature),
            other => return Verification::Unsupported(other.unwrap_or("missing").to_owned()),
        };
        if valid {
            Verification::Valid
        } else {
            Verification::Invalid
        }
    }
}

fn verify_hmac<M: Mac + KeyInit>(secret: &[u8], input: &[u8], signature: &[u8]) -> bool {
    match <M as KeyInit>::new_from_slice(secret) {
        Ok(mut mac) => {
            mac.update(input);
            mac.verify_slice(signature).is_ok()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    /// The example token from jwt.io, signed with HS256 and the secret "your-256-bit-secret".
    const JWT_IO: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    fn unsigned_token(payload: &str) -> String {
        format!(
            "{}.{}.",
            URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#),
            URL_SAFE_NO_PAD.encode(payload)
        )
    }

    #[test]
    fn decodes_header_and_payload() {
        let jwt = decode(&format!("Bearer {JWT_IO}\n")).unwrap();
        assert_eq!(jwt.algorithm(), Some("HS256"));
        assert_eq!(jwt.payload["name"], "John Doe");
        assert_eq!(
            jwt.time_claim("iat"),
            Some(Timestamp::from_second(1_516_239_022).unwrap())
        );
        assert_eq!(jwt.signature.len(), 32);
        assert!(jwt.payload_pretty().contains("\"sub\": \"1234567890\""));
    }

    #[test]
    fn verifies_hmac_signatures() {
        let jwt = decode(JWT_IO).unwrap();
        assert_eq!(jwt.verify(b"your-256-bit-secret"), Verification::Valid);
        assert_eq!(jwt.verify(b"wrong secret"), Verification::Invalid);
        let unsigned = decode(&unsigned_token("{}")).unwrap();
        assert_eq!(
            unsigned.verify(b"x"),
            Verification::Unsupported("none".into())
        );
    }

    #[test]
    fn reports_expiry() {
        let jwt = decode(&unsigned_token(r#"{"nbf":1000,"exp":2000.5}"#)).unwrap();
        let at = |seconds: i64| Timestamp::from_second(seconds).unwrap();
        assert_eq!(jwt.status(at(500)), Status::NotYetValid { from: at(1000) });
        let exp = Timestamp::from_millisecond(2_000_500).unwrap();
        assert_eq!(jwt.status(at(1500)), Status::Valid { until: Some(exp) });
        assert_eq!(jwt.status(at(3000)), Status::Expired { at: exp });
        assert_eq!(
            decode(JWT_IO).unwrap().status(at(0)),
            Status::Valid { until: None }
        );
    }

    #[test]
    fn rejects_malformed_tokens() {
        assert_eq!(decode("  "), Err(Error::Empty));
        assert_eq!(decode("a.b"), Err(Error::WrongPartCount(2)));
        assert_eq!(decode("a.b.c.d.e"), Err(Error::Encrypted));
        assert_eq!(decode("!!.e30.x"), Err(Error::InvalidBase64("header")));
        assert_eq!(
            decode(&format!("{}.e30.", URL_SAFE_NO_PAD.encode("[1]"))),
            Err(Error::InvalidJson("header"))
        );
    }
}
