//! Wi-Fi app-facing foundation.
//!
//! v0.1.18 adds real Wi-Fi credential import from WIFI.TXT and a station
//! connect state model. The credential file is intentionally tiny and local:
//!
//! ssid=YourNetwork
//! password=YourPassword

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiProvisioningStep {
    Idle,
    ImportSd,
    Connecting,
    Connected,
    Failed,
}

impl WifiProvisioningStep {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "PROV IDLE",
            Self::ImportSd => "IMPORT SD",
            Self::Connecting => "CONNECTING",
            Self::Connected => "CONNECTED",
            Self::Failed => "FAILED",
        }
    }

    pub const fn short_label(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::ImportSd => "IMPORT",
            Self::Connecting => "CONN",
            Self::Connected => "OK",
            Self::Failed => "FAIL",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiCredentialParseError {
    MissingSsid,
    SsidTooLong,
    PasswordTooLong,
}

impl WifiCredentialParseError {
    pub const fn label(self) -> &'static str {
        match self {
            Self::MissingSsid => "NO SSID",
            Self::SsidTooLong => "SSID LONG",
            Self::PasswordTooLong => "PASS LONG",
        }
    }
}

impl WifiCredentials {
    pub fn parse(input: &str) -> Result<Self, WifiCredentialParseError> {
        let mut ssid: Option<String> = None;
        let mut password: Option<String> = None;

        for raw_line in input.lines() {
            let line = raw_line.trim().trim_start_matches('\u{feff}');
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let key = key.trim().to_ascii_lowercase();
            let value = unquote(value.trim());

            match key.as_str() {
                "ssid" => ssid = Some(value.to_string()),
                "password" | "pass" | "psk" => password = Some(value.to_string()),
                _ => {}
            }
        }

        let ssid = ssid.ok_or(WifiCredentialParseError::MissingSsid)?;
        if ssid.is_empty() {
            return Err(WifiCredentialParseError::MissingSsid);
        }
        if ssid.len() > 32 {
            return Err(WifiCredentialParseError::SsidTooLong);
        }

        let password = password.unwrap_or_default();
        if password.len() > 64 {
            return Err(WifiCredentialParseError::PasswordTooLong);
        }

        Ok(Self { ssid, password })
    }
}

fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            return &value[1..value.len() - 1];
        }
    }

    value
}

pub const WIFI_STATUS_PROVISIONING_MARKER: &str =
    "v0.1.18 wifi credential import and real station connect foundation";
