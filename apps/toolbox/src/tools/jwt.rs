use super::{
    Category, Tool, ValueRow, copy_button, error_label, output_box, status_label, value_rows,
};
use egui::RichText;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use toolbox_core::encoding::decode_base64;
use toolbox_core::jwt::{self, Status, Verification};
use toolbox_core::timestamp;

const TIME_CLAIMS: [(&str, &str); 3] = [
    ("exp", "Expires"),
    ("nbf", "Not before"),
    ("iat", "Issued at"),
];

#[derive(Default)]
pub struct JwtTool {
    token: String,
    secret: String,
    secret_is_base64: bool,
}

impl Tool for JwtTool {
    fn name(&self) -> &'static str {
        "JWT Decoder"
    }

    fn description(&self) -> &'static str {
        "Decode JSON Web Tokens, check expiry and verify HMAC signatures. Tokens never leave this computer."
    }

    fn category(&self) -> Category {
        Category::Converters
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add(
            egui::TextEdit::multiline(&mut self.token)
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .desired_rows(4)
                .hint_text("Paste a JWT (eyJ…), with or without \"Bearer \""),
        );
        ui.add_space(8.0);
        if self.token.trim().is_empty() {
            return;
        }
        let jwt = match jwt::decode(&self.token) {
            Ok(jwt) => jwt,
            Err(error) => {
                error_label(ui, error);
                return;
            }
        };

        let now = Timestamp::now();
        match jwt.status(now) {
            Status::Expired { at } => status_label(
                ui,
                false,
                format!("Expired {}", timestamp::relative(at, now)),
            ),
            Status::NotYetValid { from } => status_label(
                ui,
                false,
                format!("Not valid until {}", timestamp::relative(from, now)),
            ),
            Status::Valid { until: Some(until) } => status_label(
                ui,
                true,
                format!("Valid, expires {}", timestamp::relative(until, now)),
            ),
            Status::Valid { until: None } => {
                status_label(ui, true, "Valid, with no expiry (exp) claim")
            }
        }
        ui.weak(format!(
            "Algorithm: {}",
            jwt.algorithm().unwrap_or("not specified")
        ));
        ui.add_space(8.0);

        let local = TimeZone::system();
        let claims: Vec<ValueRow> = TIME_CLAIMS
            .iter()
            .filter_map(|(claim, label)| {
                let at = jwt.time_claim(claim)?;
                let shown = format!(
                    "{} ({})",
                    timestamp::format_in_zone(at, &local),
                    timestamp::relative(at, now)
                );
                Some(ValueRow::with_copy(
                    format!("{label} ({claim})"),
                    shown,
                    at.to_string(),
                ))
            })
            .collect();
        if !claims.is_empty() {
            value_rows(ui, "jwt_claims", &claims);
            ui.add_space(8.0);
        }

        for (title, id, json) in [
            ("Header", "jwt_header", jwt.header_pretty()),
            ("Payload", "jwt_payload", jwt.payload_pretty()),
        ] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                copy_button(ui, ("copy", id), "Copy", &json);
            });
            output_box(ui, id, &json, 260.0);
            ui.add_space(8.0);
        }

        ui.label(RichText::new("Verify signature").strong());
        if !jwt.algorithm().is_some_and(|alg| alg.starts_with("HS")) {
            ui.weak(format!(
                "Only HMAC tokens (HS256, HS384, HS512) can be verified with a secret. This token uses {}.",
                jwt.algorithm().unwrap_or("no algorithm")
            ));
            return;
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.secret)
                    .password(true)
                    .desired_width(280.0)
                    .hint_text("Secret"),
            );
            ui.checkbox(&mut self.secret_is_base64, "Secret is Base64");
        });
        if self.secret.is_empty() {
            return;
        }
        let secret = if self.secret_is_base64 {
            match decode_base64(&self.secret) {
                Ok(bytes) => bytes,
                Err(error) => {
                    error_label(ui, format!("Secret: {error}"));
                    return;
                }
            }
        } else {
            self.secret.as_bytes().to_vec()
        };
        match jwt.verify(&secret) {
            Verification::Valid => status_label(ui, true, "Signature verified"),
            Verification::Invalid => status_label(ui, false, "Invalid signature for this secret"),
            Verification::Unsupported(alg) => {
                ui.weak(format!("Can't verify {alg} tokens."));
            }
        }
    }
}
