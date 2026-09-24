//! Service-account Google Sheets client (D3, D7k).
//!
//! Authenticates via Google service-account JSON key: signs an RS256 JWT
//! directly using `ring` without pulling in external OAuth2 crate trees (D7k),
//! exchanges it for an OAuth2 access token, and caches the token until 60s
//! before expiry.
//!
//! Provides the six operations required by the sheets sync runner:
//! - `list_tabs`: enumerate existing tabs by `(sheet_id, title)`
//! - `add_tab`: create a tab with frozen header row 1
//! - `read_values`: read values in A:F, padding missing cells to empty strings
//! - `write_cell`: write a single cell (for PBI ID write-back, D7a)
//! - `replace_values`: overwrite a tab in a single padded PUT without clearing (D7k)
//! - `set_status_dropdown`: clear D2:D validation then apply dropdown to task rows (D7k)

#![allow(dead_code)]

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use ring::rand::SystemRandom;
use ring::signature::{RsaKeyPair, RSA_PKCS1_SHA256};
use serde::Deserialize;
use std::fmt;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// Service-account credentials loaded from Google Cloud JSON key file.
///
/// Implements a custom [`fmt::Debug`] that explicitly redacts `private_key` (D3).
#[derive(Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub client_email: String,
    pub private_key: String,
    #[serde(default = "default_token_uri")]
    pub token_uri: String,
}

fn default_token_uri() -> String {
    "https://oauth2.googleapis.com/token".to_string()
}

impl fmt::Debug for ServiceAccountKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServiceAccountKey")
            .field("client_email", &self.client_email)
            .field("private_key", &"[REDACTED]")
            .field("token_uri", &self.token_uri)
            .finish()
    }
}

impl ServiceAccountKey {
    /// Load a service account key from a JSON file on disk.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, GSheetsError> {
        let p = path.as_ref();
        let content = std::fs::read_to_string(p).map_err(|e| GSheetsError::Io {
            path: p.to_path_buf(),
            source: e,
        })?;
        Self::from_json_str(&content)
    }

    /// Parse a service account key from a JSON string.
    pub fn from_json_str(content: &str) -> Result<Self, GSheetsError> {
        serde_json::from_str(content)
            .map_err(|e| GSheetsError::InvalidKey(format!("invalid service account JSON: {e}")))
    }

    /// Parse the PKCS#8 PEM private key into a `ring::signature::RsaKeyPair`.
    ///
    /// Strips BEGIN/END lines and decodes standard base64 before passing DER bytes
    /// to `ring::signature::RsaKeyPair::from_pkcs8`.
    pub fn parse_key_pair(&self) -> Result<RsaKeyPair, GSheetsError> {
        let b64: String = self
            .private_key
            .lines()
            .filter(|line| !line.trim().starts_with("-----"))
            .collect();
        let der = STANDARD
            .decode(b64.trim().as_bytes())
            .map_err(|_| GSheetsError::InvalidKey("failed to base64-decode private key PEM".into()))?;
        RsaKeyPair::from_pkcs8(&der)
            .map_err(|_| GSheetsError::InvalidKey("invalid PKCS#8 RSA key format".into()))
    }
}

/// Errors originating from the Google Sheets client.
///
/// Error messages and Debug representations never print `private_key` or auth tokens.
#[derive(Debug, thiserror::Error)]
pub enum GSheetsError {
    #[error("Google API error ({status}): {message}")]
    Api {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error on {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("Invalid service account key: {0}")]
    InvalidKey(String),
}

#[derive(Clone)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

/// Google Sheets API client backed by a service-account JWT grant.
#[derive(Clone)]
pub struct SheetsClient {
    key: ServiceAccountKey,
    http: reqwest::Client,
    token_url: String,
    api_base: String,
    token_cache: Arc<Mutex<Option<CachedToken>>>,
}

impl SheetsClient {
    /// Construct a new Sheets client with default Google production endpoints.
    pub fn new(key: ServiceAccountKey, http: reqwest::Client) -> Self {
        let token_url = if key.token_uri.is_empty() {
            default_token_uri()
        } else {
            key.token_uri.clone()
        };
        Self {
            key,
            http,
            token_url,
            api_base: "https://sheets.googleapis.com/v4".to_string(),
            token_cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Override the OAuth2 token endpoint URL (for mock testing).
    pub fn with_token_url(mut self, url: impl Into<String>) -> Self {
        self.token_url = url.into();
        self
    }

    /// Override the Google Sheets API base URL (for mock testing).
    pub fn with_api_base(mut self, url: impl Into<String>) -> Self {
        self.api_base = url.into();
        self
    }

    /// Override both token and API base endpoints (for mock testing).
    pub fn with_endpoints(mut self, token_url: impl Into<String>, api_base: impl Into<String>) -> Self {
        self.token_url = token_url.into();
        self.api_base = api_base.into();
        self
    }

    /// Reference to the underlying service account key.
    pub fn key(&self) -> &ServiceAccountKey {
        &self.key
    }

    /// Build an RS256-signed JWT assertion for OAuth2 token exchange (D7k).
    pub fn build_jwt(&self) -> Result<String, GSheetsError> {
        let header = serde_json::json!({
            "alg": "RS256",
            "typ": "JWT"
        });
        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&header)?.as_bytes());

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let claims = serde_json::json!({
            "iss": self.key.client_email,
            "scope": "https://www.googleapis.com/auth/spreadsheets",
            "aud": self.token_url,
            "iat": now,
            "exp": now + 3600
        });
        let claims_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_string(&claims)?.as_bytes());

        let signing_input = format!("{header_b64}.{claims_b64}");
        let key_pair = self.key.parse_key_pair()?;
        let rng = SystemRandom::new();
        let mut sig = vec![0; key_pair.public().modulus_len()];
        key_pair
            .sign(&RSA_PKCS1_SHA256, &rng, signing_input.as_bytes(), &mut sig)
            .map_err(|_| GSheetsError::InvalidKey("failed to sign JWT with RSA key".into()))?;
        let sig_b64 = URL_SAFE_NO_PAD.encode(&sig);

        Ok(format!("{signing_input}.{sig_b64}"))
    }

    /// Retrieve an access token, reusing the cached token if valid or exchanging a new JWT assertion.
    ///
    /// Caches the token until 60 seconds before its expiry.
    pub async fn get_access_token(&self) -> Result<String, GSheetsError> {
        let mut cache = self.token_cache.lock().await;
        let now = Instant::now();
        if let Some(cached) = &*cache {
            if now < cached.expires_at {
                return Ok(cached.access_token.clone());
            }
        }

        let jwt = self.build_jwt()?;
        let resp = self
            .http
            .post(&self.token_url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: u64,
        }

        let token_resp: TokenResponse = resp.json().await?;
        let valid_secs = token_resp.expires_in.saturating_sub(60);
        let expires_at = now + Duration::from_secs(valid_secs);

        *cache = Some(CachedToken {
            access_token: token_resp.access_token.clone(),
            expires_at,
        });

        Ok(token_resp.access_token)
    }

    /// List existing tabs in the spreadsheet as `(sheet_id, title)`.
    ///
    /// GET `spreadsheets/{id}?fields=sheets.properties`
    pub async fn list_tabs(&self, spreadsheet_id: &str) -> Result<Vec<(i64, String)>, GSheetsError> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/spreadsheets/{}?fields=sheets.properties",
            self.api_base, spreadsheet_id
        );
        let resp = self.http.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        #[derive(Deserialize)]
        struct SheetProperties {
            #[serde(rename = "sheetId")]
            sheet_id: i64,
            #[serde(default)]
            title: String,
        }

        #[derive(Deserialize)]
        struct SheetItem {
            properties: SheetProperties,
        }

        #[derive(Deserialize)]
        struct SpreadsheetResponse {
            #[serde(default)]
            sheets: Vec<SheetItem>,
        }

        let body: SpreadsheetResponse = resp.json().await?;
        let tabs = body
            .sheets
            .into_iter()
            .map(|s| (s.properties.sheet_id, s.properties.title))
            .collect();
        Ok(tabs)
    }

    /// Add a new tab with frozen row 1, returning its numeric `sheetId`.
    ///
    /// POST `spreadsheets/{id}:batchUpdate` with an `addSheet` request.
    pub async fn add_tab(&self, spreadsheet_id: &str, title: &str) -> Result<i64, GSheetsError> {
        let token = self.get_access_token().await?;
        let url = format!("{}/spreadsheets/{}:batchUpdate", self.api_base, spreadsheet_id);
        let payload = serde_json::json!({
            "requests": [
                {
                    "addSheet": {
                        "properties": {
                            "title": title,
                            "gridProperties": {
                                "frozenRowCount": 1
                            }
                        }
                    }
                }
            ]
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        #[derive(Deserialize)]
        struct AddSheetProperties {
            #[serde(rename = "sheetId")]
            sheet_id: i64,
        }

        #[derive(Deserialize)]
        struct AddSheetReply {
            properties: AddSheetProperties,
        }

        #[derive(Deserialize)]
        struct BatchReply {
            #[serde(rename = "addSheet")]
            add_sheet: Option<AddSheetReply>,
        }

        #[derive(Deserialize)]
        struct BatchUpdateResponse {
            #[serde(default)]
            replies: Vec<BatchReply>,
        }

        let body: BatchUpdateResponse = resp.json().await?;
        let sheet_id = body
            .replies
            .into_iter()
            .find_map(|r| r.add_sheet.map(|a| a.properties.sheet_id))
            .ok_or_else(|| GSheetsError::Api {
                status: reqwest::StatusCode::OK,
                message: "addSheet reply missing in batchUpdate response".into(),
            })?;

        Ok(sheet_id)
    }

    /// Read values in columns A:F of the given tab title.
    ///
    /// Missing cells in any row are padded with empty strings to guarantee 6 columns.
    /// GET `spreadsheets/{id}/values/'title'!A:F`
    pub async fn read_values(
        &self,
        spreadsheet_id: &str,
        title: &str,
    ) -> Result<Vec<Vec<String>>, GSheetsError> {
        let token = self.get_access_token().await?;
        let encoded_range = quote_a1_range(title, "A:F");
        let url = format!(
            "{}/spreadsheets/{}/values/{}",
            self.api_base, spreadsheet_id, encoded_range
        );
        let resp = self.http.get(&url).bearer_auth(token).send().await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        #[derive(Deserialize)]
        struct ValueRange {
            #[serde(default)]
            values: Vec<Vec<serde_json::Value>>,
        }

        let body: ValueRange = resp.json().await?;
        let rows = body
            .values
            .into_iter()
            .map(|row| {
                let mut row_strings: Vec<String> = row
                    .into_iter()
                    .map(|v| match v {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Null => String::new(),
                        other => other.to_string(),
                    })
                    .collect();
                while row_strings.len() < 6 {
                    row_strings.push(String::new());
                }
                row_strings
            })
            .collect();

        Ok(rows)
    }

    /// Write a single cell with raw value input option (used for D7a ID write-back).
    ///
    /// PUT `spreadsheets/{id}/values/'title'!{a1}?valueInputOption=RAW`
    pub async fn write_cell(
        &self,
        spreadsheet_id: &str,
        title: &str,
        a1: &str,
        value: &str,
    ) -> Result<(), GSheetsError> {
        let token = self.get_access_token().await?;
        let encoded_range = quote_a1_range(title, a1);
        let url = format!(
            "{}/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.api_base, spreadsheet_id, encoded_range
        );
        let unescaped_range = format!("'{}'!{}", title.replace('\'', "''"), a1);
        let payload = serde_json::json!({
            "range": unescaped_range,
            "majorDimension": "ROWS",
            "values": [[value]]
        });

        let resp = self
            .http
            .put(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        Ok(())
    }

    /// Replace the entire contents of a tab starting at A1 in a single PUT request (D7k).
    ///
    /// Pads the provided rows with blank rows up to `previous_row_count` so trailing rows
    /// are blanked out without requiring a separate `clear` call.
    pub async fn replace_values(
        &self,
        spreadsheet_id: &str,
        title: &str,
        rows: &[Vec<String>],
        previous_row_count: usize,
    ) -> Result<(), GSheetsError> {
        let token = self.get_access_token().await?;
        let encoded_range = quote_a1_range(title, "A1");
        let url = format!(
            "{}/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.api_base, spreadsheet_id, encoded_range
        );

        let mut padded_rows = rows.to_vec();
        let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(6).max(6);
        while padded_rows.len() < previous_row_count {
            padded_rows.push(vec![String::new(); col_count]);
        }

        let unescaped_range = format!("'{}'!A1", title.replace('\'', "''"));
        let payload = serde_json::json!({
            "range": unescaped_range,
            "majorDimension": "ROWS",
            "values": padded_rows
        });

        let resp = self
            .http
            .put(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        Ok(())
    }

    /// Configure status validation on task rows as a single batchUpdate (D7k).
    ///
    /// First clears data validation on D2:D by sending a `setDataValidation` request
    /// without a rule, then applies `ONE_OF_LIST` validation (proposed, in-flight, parked,
    /// done, declined) on column D for each index in `task_row_indices`.
    pub async fn set_status_dropdown(
        &self,
        spreadsheet_id: &str,
        sheet_id: i64,
        task_row_indices: &[usize],
    ) -> Result<(), GSheetsError> {
        let token = self.get_access_token().await?;
        let url = format!("{}/spreadsheets/{}:batchUpdate", self.api_base, spreadsheet_id);

        let mut requests = Vec::with_capacity(1 + task_row_indices.len());

        // 1. Clear validation over D2:D
        requests.push(serde_json::json!({
            "setDataValidation": {
                "range": {
                    "sheetId": sheet_id,
                    "startRowIndex": 1,
                    "startColumnIndex": 3,
                    "endColumnIndex": 4
                }
            }
        }));

        // 2. Set dropdown on each task row's D cell
        for &row_idx in task_row_indices {
            requests.push(serde_json::json!({
                "setDataValidation": {
                    "range": {
                        "sheetId": sheet_id,
                        "startRowIndex": row_idx,
                        "endRowIndex": row_idx + 1,
                        "startColumnIndex": 3,
                        "endColumnIndex": 4
                    },
                    "rule": {
                        "condition": {
                            "type": "ONE_OF_LIST",
                            "values": [
                                {"userEnteredValue": "proposed"},
                                {"userEnteredValue": "in-flight"},
                                {"userEnteredValue": "parked"},
                                {"userEnteredValue": "done"},
                                {"userEnteredValue": "declined"}
                            ]
                        },
                        "showCustomUi": true,
                        "strict": true
                    }
                }
            }));
        }

        let payload = serde_json::json!({
            "requests": requests
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::extract_error(resp).await);
        }

        Ok(())
    }

    /// Extract an error from an HTTP response, parsing Google's `error.message` if present.
    async fn extract_error(resp: reqwest::Response) -> GSheetsError {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let message = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
            json.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| text.clone())
        } else {
            text
        };
        GSheetsError::Api { status, message }
    }
}

/// Quote a tab title in A1 notation and URL-encode the range.
///
/// Tab titles are enclosed in single quotes with embedded single quotes doubled,
/// then concatenated with `!{suffix}` and RFC 3986 percent-encoded.
pub fn quote_a1_range(title: &str, range_suffix: &str) -> String {
    let escaped_title = title.replace('\'', "''");
    let raw = format!("'{escaped_title}'!{range_suffix}");
    url_encode(&raw)
}

/// RFC 3986 percent-encode a string.
pub fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b => {
                use std::fmt::Write;
                let _ = write!(encoded, "%{:02X}", b);
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::{Path as AxPath, Request};
    use axum::http::StatusCode;
    use axum::response::Json;
    use axum::routing::{get, post};
    use axum::Router;
    use ring::signature::{UnparsedPublicKey, RSA_PKCS1_2048_8192_SHA256};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub const TEST_PKCS8_PEM: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC5w1is4/YKniI0
+/EuH5b248KiYb5KigbWtNWENpd95hUqGiw4Bni/lC/amlFV3jnOON9uu0aBDQgH
ZODrzdhcYXAGMn/Cz9mtjggM3JxM2fdDDojDSP9fiCqMy69VXlpTW6n9uBbKYo8l
oLs5L70YYT0zwBulUwBtY0wMeccsrS8AB3yyXyiRm/tP/Giapb/TEJl/2WMWPkJL
B1JdaJZf/8SMRx0hvpeFuBCV52iiouLUaMHR+tgapHEEv+TEqmHbBWwfczVmNiHX
maJ7mdAax8/B5xdoLXPcqYKCUHJwg4EgCtS8hq0Rofvy5/f4v1wXLVu3A30DWQJ1
l7jBzr1nAgMBAAECggEAUcfCXjrtsrbzcEgKYbfeikqzATHuyHoDkLqD1XUDNJX6
c1KabP/MYOsoNiEB9b+jyzoWg3VHEMFf64Fp0+YHfoQXAWNyA53lwxZoTOFCnkLH
LGpVo0Obyb70nI9/XSHwugn8yIom3mqNgo9EXWS65uTIQOQuR5ZG9eFlNYjXOr8x
TJsdaFFCDjxaoqt8s1iWl1FoTXB2gc0E4Y9xMeDPzLfyHu4Tj0jS3O1cC2lD1rzu
fp+f9lue2aKtSuyE62TahcVVlSkNorRBBEkFdQDIzA3dejldUU8UZfn6Vy23j5jZ
/WEBGyIy1xQPSI80GlEvGh+jCZ9Jnz+Y2bE1XbWuoQKBgQD+U9Wy6NcalcNGfeQ0
3sUD3NA5FY/RABPDLN5g01d1tBZ7C5Nl7PWNyvzSFSOhsZ0XSZPzZ11SG//M27Mh
xBCIWkrTtHyuoZRu+augpu3pLYgtYR6Mc6eZAuAjouYAO/e9sRkYAvuig78huN1w
AdaQrLllvAcKFS76JUnzErl/1QKBgQC6/BUFs6447tXS4/+38zKYitGccQYjJ9hP
W+ByCs/qNsqrFCrWo7pulKWoHmgAuLnwWdWQbkZPZc2ktslmehKTGZeJGU8oyfBH
gBuQEAdiEyE9cIQwtk2Dx+C4ntIgvZxAu+YS7q4W28Oibbrq+OKtboKbyRCCIjgo
Ppxb0owiSwKBgQCfaf8eKJKECHGhYtHDTgth82vVDC3cTFcW1K4DNRWyIpKgx825
asE0ApNcP4HZHVKhQS5quFSnXgOF7gQnWxUd0rLqZkxszb0Tno20ok2oBjFAeIrE
cM4Ru9qwm6/dX7lW/oYDJgOCh2ArjAvI5G4QuyNuoGAtMsACbZMfuJHrjQKBgHaQ
aXQYHx0StJro37lwiGCZhZhBQrcACPUpOQSXPocw/wnD2/YGE60SrIgR37i1aZGy
JO1UP1kRIHiVPtewCB8mVyQj3EHs6mMsJE4RG91ga+9dbTTGJBivoRrv+iWwlWem
vZi7p7/u0foowiFEd9eo/0BxGfqqYBmEufivZRRJAoGBAOIq1N3FRncJ0RDbq6du
UrcBjX1JqiNhZ/CWZXfLTjKbGCuDb7F9RwXjIVHzX5LA38M1mVaFR226ewBsE2Zf
3i+mY8Z1GR2tTlC6wNO+2enqY5zsBLv3di27mQm2UL/5ry2jO+LERXZl9XM1pAj+
sgjI3687NnKVf2hyV4epQthL
-----END PRIVATE KEY-----
"#;

    fn test_key() -> ServiceAccountKey {
        ServiceAccountKey {
            client_email: "test-agent@example.iam.gserviceaccount.com".to_string(),
            private_key: TEST_PKCS8_PEM.to_string(),
            token_uri: "http://127.0.0.1:0/token".to_string(),
        }
    }

    #[derive(Default, Clone)]
    struct MockServerState {
        token_calls: Arc<AtomicUsize>,
        last_jwt: Arc<Mutex<Option<String>>>,
        last_batch_requests: Arc<Mutex<Option<serde_json::Value>>>,
        last_put_values: Arc<Mutex<Option<serde_json::Value>>>,
        last_requested_path: Arc<Mutex<Option<String>>>,
        clear_calls: Arc<AtomicUsize>,
    }

    async fn start_mock_server(state: MockServerState) -> (String, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route(
                "/token",
                post({
                    let st = state.clone();
                    move |body: String| {
                        let st = st.clone();
                        async move {
                            st.token_calls.fetch_add(1, Ordering::SeqCst);
                            let params: HashMap<String, String> = body
                                .split('&')
                                .filter_map(|pair| {
                                    let mut parts = pair.splitn(2, '=');
                                    let k = parts.next()?;
                                    let v = parts.next()?;
                                    Some((k.to_string(), v.to_string()))
                                })
                                .collect();
                            let assertion = params.get("assertion").cloned().unwrap_or_default();
                            *st.last_jwt.lock().await = Some(assertion);

                            Json(serde_json::json!({
                                "access_token": "mock-access-token-xyz",
                                "expires_in": 3600,
                                "token_type": "Bearer"
                            }))
                        }
                    }
                }),
            )
            .route(
                "/spreadsheets/:id",
                get({
                    let st = state.clone();
                    move |AxPath(id): AxPath<String>| {
                        let _ = (st, id);
                        async move {
                            Json(serde_json::json!({
                                "sheets": [
                                    {
                                        "properties": {
                                            "sheetId": 0,
                                            "title": "Sheet1"
                                        }
                                    },
                                    {
                                        "properties": {
                                            "sheetId": 42,
                                            "title": "Tasks"
                                        }
                                    }
                                ]
                            }))
                        }
                    }
                })
                .post({
                    let st = state.clone();
                    move |AxPath(id): AxPath<String>, Json(body): Json<serde_json::Value>| {
                        let st = st.clone();
                        let _ = id;
                        async move {
                            *st.last_batch_requests.lock().await = Some(body.clone());
                            Json(serde_json::json!({
                                "replies": [
                                    {
                                        "addSheet": {
                                            "properties": {
                                                "sheetId": 12345,
                                                "title": "NewTab"
                                            }
                                        }
                                    }
                                ]
                            }))
                        }
                    }
                }),
            )
            .route(
                "/spreadsheets/:id/values/*range",
                get({
                    let st = state.clone();
                    move |AxPath((id, range)): AxPath<(String, String)>, req: Request| {
                        let st = st.clone();
                        let _ = id;
                        async move {
                            *st.last_requested_path.lock().await = Some(req.uri().to_string());
                            let _ = range;
                            Json(serde_json::json!({
                                "range": "Tasks!A:F",
                                "majorDimension": "ROWS",
                                "values": [
                                    ["ID", "Type", "Title", "Status", "Description", "Feature"],
                                    ["1", "task"],
                                    []
                                ]
                            }))
                        }
                    }
                })
                .put({
                    let st = state.clone();
                    move |AxPath((id, range)): AxPath<(String, String)>, Json(body): Json<serde_json::Value>| {
                        let st = st.clone();
                        let _ = (id, range);
                        async move {
                            *st.last_put_values.lock().await = Some(body);
                            Json(serde_json::json!({
                                "updatedRows": 1
                            }))
                        }
                    }
                })
                .post({
                    let st = state.clone();
                    move || {
                        let st = st.clone();
                        async move {
                            st.clear_calls.fetch_add(1, Ordering::SeqCst);
                            Json(serde_json::json!({}))
                        }
                    }
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let addr = format!("http://127.0.0.1:{port}");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        (addr, handle)
    }

    #[tokio::test]
    async fn test_jwt_verification_and_claims() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let mut key = test_key();
        key.token_uri = format!("{base}/token");
        let client = SheetsClient::new(key.clone(), reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        // Make an API call to trigger token acquisition
        let tabs = client.list_tabs("test-spread").await.unwrap();
        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs[0], (0, "Sheet1".to_string()));
        assert_eq!(tabs[1], (42, "Tasks".to_string()));

        // Inspect captured JWT
        let jwt = state.last_jwt.lock().await.clone().expect("JWT was posted");
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT must consist of three parts");

        // Header check
        let header_bytes = URL_SAFE_NO_PAD.decode(parts[0]).unwrap();
        let header_val: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();
        assert_eq!(header_val["alg"], "RS256");
        assert_eq!(header_val["typ"], "JWT");

        // Claims check
        let claims_bytes = URL_SAFE_NO_PAD.decode(parts[1]).unwrap();
        let claims_val: serde_json::Value = serde_json::from_slice(&claims_bytes).unwrap();
        assert_eq!(claims_val["iss"], key.client_email);
        assert_eq!(
            claims_val["scope"],
            "https://www.googleapis.com/auth/spreadsheets"
        );
        assert_eq!(claims_val["aud"], format!("{base}/token"));
        let iat = claims_val["iat"].as_u64().unwrap();
        let exp = claims_val["exp"].as_u64().unwrap();
        assert_eq!(exp, iat + 3600);

        // Verify signature with ring UnparsedPublicKey using public key
        let key_pair = key.parse_key_pair().unwrap();
        let pub_key_bytes = key_pair.public().as_ref();
        let sig_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let signing_input = format!("{}.{}", parts[0], parts[1]);

        let verifier = UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA256, pub_key_bytes);
        verifier
            .verify(signing_input.as_bytes(), &sig_bytes)
            .expect("JWT signature must verify via public key");
    }

    #[tokio::test]
    async fn test_token_cached_across_two_calls() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let key = test_key();
        let client = SheetsClient::new(key, reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        // First call fetches token
        client.list_tabs("s1").await.unwrap();
        assert_eq!(state.token_calls.load(Ordering::SeqCst), 1);

        // Second call within lifetime reuses cached token
        client.list_tabs("s1").await.unwrap();
        assert_eq!(
            state.token_calls.load(Ordering::SeqCst),
            1,
            "Second call within lifetime must reuse cached token"
        );
    }

    #[tokio::test]
    async fn test_read_values_pads_missing_cells() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let key = test_key();
        let client = SheetsClient::new(key, reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        let rows = client.read_values("s1", "Tasks").await.unwrap();
        assert_eq!(rows.len(), 3);
        // Header row (6 cells)
        assert_eq!(rows[0].len(), 6);
        assert_eq!(rows[0][0], "ID");
        // Row 1 (was 2 cells, padded to 6 with empty strings)
        assert_eq!(rows[1].len(), 6);
        assert_eq!(rows[1], vec!["1", "task", "", "", "", ""]);
        // Row 2 (was 0 cells, padded to 6 empty strings)
        assert_eq!(rows[2].len(), 6);
        assert_eq!(rows[2], vec!["", "", "", "", "", ""]);
    }

    #[tokio::test]
    async fn test_replace_values_sends_one_put_covering_previous_row_count() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let key = test_key();
        let client = SheetsClient::new(key, reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        let rows = vec![
            vec!["A".into(), "B".into(), "C".into(), "D".into(), "E".into(), "F".into()],
            vec!["1".into(), "task".into(), "title".into(), "done".into(), "cos".into(), "feat".into()],
        ];

        // Replace with previous_row_count = 5 (should pad 3 empty rows)
        client.replace_values("s1", "Tasks", &rows, 5).await.unwrap();

        assert_eq!(
            state.clear_calls.load(Ordering::SeqCst),
            0,
            "Rewriting a tab must not issue a clear request (D7k)"
        );

        let put_body = state.last_put_values.lock().await.clone().unwrap();
        let values = put_body["values"].as_array().unwrap();
        assert_eq!(values.len(), 5, "PUT values must cover the previous row count");
        // First 2 rows match input
        assert_eq!(values[0][0], "A");
        assert_eq!(values[1][0], "1");
        // Last 3 rows are padded empty strings
        for i in 2..5 {
            let row = values[i].as_array().unwrap();
            assert_eq!(row.len(), 6);
            assert!(row.iter().all(|c| c.as_str() == Some("")));
        }
    }

    #[tokio::test]
    async fn test_set_status_dropdown_sends_clearing_request_before_rules() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let key = test_key();
        let client = SheetsClient::new(key, reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        // Call dropdown validation on row indices 1 and 3 (0-based sheet row indices)
        client.set_status_dropdown("s1", 42, &[1, 3]).await.unwrap();

        let batch_body = state.last_batch_requests.lock().await.clone().unwrap();
        let requests = batch_body["requests"].as_array().unwrap();
        assert_eq!(requests.len(), 3);

        // First request must be the clearing request on D2:D (no rule)
        let first = &requests[0]["setDataValidation"];
        assert_eq!(first["range"]["sheetId"], 42);
        assert_eq!(first["range"]["startRowIndex"], 1); // Row 2 in 0-based
        assert_eq!(first["range"]["startColumnIndex"], 3); // Col D (0-based)
        assert_eq!(first["range"]["endColumnIndex"], 4);
        assert!(first.get("rule").is_none() || first["rule"].is_null());

        // Subsequent requests must have ONE_OF_LIST rules
        let expected_statuses = vec!["proposed", "in-flight", "parked", "done", "declined"];
        for (req, expected_row) in requests[1..].iter().zip([1, 3]) {
            let val = &req["setDataValidation"];
            assert_eq!(val["range"]["sheetId"], 42);
            assert_eq!(val["range"]["startRowIndex"], expected_row);
            assert_eq!(val["range"]["endRowIndex"], expected_row + 1);
            assert_eq!(val["range"]["startColumnIndex"], 3);
            assert_eq!(val["range"]["endColumnIndex"], 4);

            let condition = &val["rule"]["condition"];
            assert_eq!(condition["type"], "ONE_OF_LIST");
            let list_values: Vec<&str> = condition["values"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v["userEnteredValue"].as_str().unwrap())
                .collect();
            assert_eq!(list_values, expected_statuses);
            assert_eq!(val["rule"]["showCustomUi"], true);
        }
    }

    #[tokio::test]
    async fn test_error_and_debug_never_contain_private_key() {
        let key = test_key();
        let debug_str = format!("{key:?}");
        assert!(
            debug_str.contains("[REDACTED]"),
            "Debug output must redact private_key"
        );
        assert!(
            !debug_str.contains("BEGIN PRIVATE KEY"),
            "Debug output must never contain private_key bytes"
        );
        assert!(
            !debug_str.contains("MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQC5w1is4"),
            "Debug output must never contain private_key base64"
        );

        // Test error formatting
        let err = GSheetsError::InvalidKey("bad key pem format".into());
        let err_debug = format!("{err:?}");
        let err_display = format!("{err}");
        assert!(!err_debug.contains("MIIEvgIBADAN"));
        assert!(!err_display.contains("MIIEvgIBADAN"));

        // Server error response mapping
        let app = Router::new().route(
            "/error-endpoint",
            get(|| async {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": {
                            "code": 400,
                            "message": "Invalid range specified",
                            "status": "INVALID_ARGUMENT"
                        }
                    })),
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let resp = reqwest::Client::new()
            .get(format!("http://127.0.0.1:{port}/error-endpoint"))
            .send()
            .await
            .unwrap();

        let api_err = SheetsClient::extract_error(resp).await;
        let err_str = format!("{api_err:?}");
        assert!(err_str.contains("Invalid range specified"));
        assert!(!err_str.contains("MIIEvgIBADAN"));
    }

    #[tokio::test]
    async fn test_tab_title_with_quote_is_escaped() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let key = test_key();
        let client = SheetsClient::new(key, reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        // Tab title containing single and double quotes
        let tab_title = "Bob's \"Special\" Tab";
        let _ = client.read_values("s1", tab_title).await;

        let requested = state.last_requested_path.lock().await.clone().unwrap();
        // Embedded single quote must be doubled to ''
        // In A1 notation: 'Bob''s "Special" Tab'!A:F
        // Encoded: %27Bob%27%27s%20%22Special%22%20Tab%27%21A%3AF
        assert!(
            requested.contains("%27Bob%27%27s%20%22Special%22%20Tab%27%21A%3AF"),
            "Tab title with quote must be properly escaped in A1 range and URL-encoded. Received: {requested}"
        );
    }

    #[tokio::test]
    async fn test_add_tab_and_write_cell() {
        let state = MockServerState::default();
        let (base, _server) = start_mock_server(state.clone()).await;

        let key = test_key();
        let client = SheetsClient::new(key, reqwest::Client::new())
            .with_endpoints(format!("{base}/token"), base.clone());

        let sheet_id = client.add_tab("s1", "NewTab").await.unwrap();
        assert_eq!(sheet_id, 12345);

        // write_cell single cell PUT
        client.write_cell("s1", "Tasks", "A2", "p-100").await.unwrap();
        let put_body = state.last_put_values.lock().await.clone().unwrap();
        assert_eq!(put_body["values"], serde_json::json!([["p-100"]]));
    }
}
