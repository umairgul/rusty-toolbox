//! JSON Web Token decoding. Times are shown in this Mac's time zone; see `timestamps.rs` for
//! why jiff's types are only ever inferred here.

use crate::ToolError;
use crate::timestamps::current_time;
use toolbox_core::encoding::decode_base64;
use toolbox_core::jwt::{self, Status, Verification};
use toolbox_core::timestamp;

/// The NumericDate claims shown with their dates, and their labels.
const TIME_CLAIMS: [(&str, &str); 3] = [
    ("exp", "Expires"),
    ("nbf", "Not before"),
    ("iat", "Issued at"),
];

/// A green or red line about the token, e.g. "Valid, expires in 3 hours".
#[derive(uniffi::Record)]
pub struct JwtStatus {
    pub ok: bool,
    pub message: String,
}

/// A time claim, e.g. label "Expires (exp)", shown "2023-11-14 17:13:20 EST (-05:00) (in
/// 3 hours)", copied "2023-11-14T22:13:20Z".
#[derive(uniffi::Record)]
pub struct JwtClaim {
    pub label: String,
    pub shown: String,
    pub copy: String,
}

#[derive(uniffi::Record)]
pub struct JwtDecoded {
    /// The `alg` header, if any.
    pub algorithm: Option<String>,
    /// e.g. "Algorithm: HS256" or "Algorithm: not specified".
    pub algorithm_description: String,
    /// The header and payload as indented JSON.
    pub header: String,
    pub payload: String,
    /// Expiry, relative to now.
    pub status: JwtStatus,
    /// The `exp`, `nbf` and `iat` claims present, in that order.
    pub claims: Vec<JwtClaim>,
    /// Whether the signature can be checked with a secret (HS256, HS384, HS512).
    pub verifiable: bool,
    /// Why it can't, when it can't.
    pub verify_note: Option<String>,
}

#[derive(uniffi::Enum)]
pub enum JwtVerification {
    Valid {
        message: String,
    },
    Invalid {
        message: String,
    },
    /// The secret should be Base64 but isn't.
    InvalidSecret {
        message: String,
    },
    /// Not an HMAC token: a note rather than a verdict.
    Unsupported {
        message: String,
    },
}

/// Decodes a JWT, with or without a `Bearer ` prefix, and checks its expiry against now.
#[uniffi::export]
pub fn jwt_decode(token: &str) -> Result<JwtDecoded, ToolError> {
    let jwt = jwt::decode(token).map_err(ToolError::new)?;
    let now = current_time(timestamp::unix).ok_or_else(|| ToolError::new("No system clock"))?;
    let status = match jwt.status(now) {
        Status::Expired { at } => JwtStatus {
            ok: false,
            message: format!("Expired {}", timestamp::relative(at, now)),
        },
        Status::NotYetValid { from } => JwtStatus {
            ok: false,
            message: format!("Not valid until {}", timestamp::relative(from, now)),
        },
        Status::Valid { until: Some(until) } => JwtStatus {
            ok: true,
            message: format!("Valid, expires {}", timestamp::relative(until, now)),
        },
        Status::Valid { until: None } => JwtStatus {
            ok: true,
            message: "Valid, with no expiry (exp) claim".into(),
        },
    };

    let local = timestamp::system_zone_name()
        .and_then(|name| now.in_tz(&name).ok())
        .or_else(|| now.in_tz("UTC").ok())
        .map(|zoned| zoned.time_zone().clone());
    let claims = TIME_CLAIMS
        .iter()
        .filter_map(|(claim, label)| {
            let at = jwt.time_claim(claim)?;
            let date = match &local {
                Some(local) => timestamp::format_in_zone(at, local),
                None => at.to_string(),
            };
            Some(JwtClaim {
                label: format!("{label} ({claim})"),
                shown: format!("{date} ({})", timestamp::relative(at, now)),
                copy: at.to_string(),
            })
        })
        .collect();

    let algorithm = jwt.algorithm().map(str::to_owned);
    let verifiable = algorithm
        .as_deref()
        .is_some_and(|alg| alg.starts_with("HS"));
    Ok(JwtDecoded {
        algorithm_description: format!(
            "Algorithm: {}",
            algorithm.as_deref().unwrap_or("not specified")
        ),
        verify_note: (!verifiable).then(|| {
            format!(
                "Only HMAC tokens (HS256, HS384, HS512) can be verified with a secret. This token uses {}.",
                algorithm.as_deref().unwrap_or("no algorithm")
            )
        }),
        algorithm,
        header: jwt.header_pretty(),
        payload: jwt.payload_pretty(),
        status,
        claims,
        verifiable,
    })
}

/// Checks the token's HMAC signature against `secret`, which is Base64 when
/// `secret_is_base64`, otherwise UTF-8 text.
#[uniffi::export]
pub fn jwt_verify(
    token: &str,
    secret: &str,
    secret_is_base64: bool,
) -> Result<JwtVerification, ToolError> {
    let jwt = jwt::decode(token).map_err(ToolError::new)?;
    let secret = if secret_is_base64 {
        match decode_base64(secret) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(JwtVerification::InvalidSecret {
                    message: format!("Secret: {error}"),
                });
            }
        }
    } else {
        secret.as_bytes().to_vec()
    };
    Ok(match jwt.verify(&secret) {
        Verification::Valid => JwtVerification::Valid {
            message: "Signature verified".into(),
        },
        Verification::Invalid => JwtVerification::Invalid {
            message: "Invalid signature for this secret".into(),
        },
        Verification::Unsupported(alg) => JwtVerification::Unsupported {
            message: format!("Can't verify {alg} tokens."),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example token from jwt.io, signed with HS256 and the secret "your-256-bit-secret".
    const JWT_IO: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[test]
    fn decodes_claims_and_status() {
        let decoded = jwt_decode(JWT_IO).unwrap();
        assert_eq!(decoded.algorithm.as_deref(), Some("HS256"));
        assert_eq!(decoded.algorithm_description, "Algorithm: HS256");
        assert!(decoded.status.ok);
        assert_eq!(decoded.status.message, "Valid, with no expiry (exp) claim");
        assert_eq!(decoded.claims.len(), 1);
        assert_eq!(decoded.claims[0].label, "Issued at (iat)");
        assert_eq!(decoded.claims[0].copy, "2018-01-18T01:30:22Z");
        assert!(decoded.claims[0].shown.ends_with("years ago)"));
        assert!(decoded.verifiable);
        assert!(decoded.verify_note.is_none());
        assert!(decoded.payload.contains("\"name\": \"John Doe\""));
    }

    #[test]
    fn verifies_secrets() {
        let verify = |secret, base64| jwt_verify(JWT_IO, secret, base64).unwrap();
        assert!(matches!(
            verify("your-256-bit-secret", false),
            JwtVerification::Valid { .. }
        ));
        assert!(matches!(
            verify("eW91ci0yNTYtYml0LXNlY3JldA==", true),
            JwtVerification::Valid { .. }
        ));
        assert!(matches!(
            verify("wrong", false),
            JwtVerification::Invalid { .. }
        ));
        let JwtVerification::InvalidSecret { message } = verify("!!", true) else {
            panic!("expected InvalidSecret")
        };
        assert_eq!(message, "Secret: Not valid Base64");
    }

    #[test]
    fn explains_unverifiable_tokens() {
        // {"alg":"none"}.{"exp":1}.
        let decoded = jwt_decode("eyJhbGciOiJub25lIn0.eyJleHAiOjF9.").unwrap();
        assert!(!decoded.status.ok);
        assert!(decoded.status.message.starts_with("Expired "));
        assert!(!decoded.verifiable);
        assert_eq!(
            decoded.verify_note.as_deref(),
            Some(
                "Only HMAC tokens (HS256, HS384, HS512) can be verified with a secret. This token uses none."
            )
        );
        assert!(jwt_decode("a.b").is_err());
    }
}
