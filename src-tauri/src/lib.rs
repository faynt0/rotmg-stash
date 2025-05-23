// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod devicetoken_script;
mod util;

use base64::{engine::general_purpose, Engine as _};
use dirs;
use log;
use regex::Regex;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use tauri_plugin_log;
use url::form_urlencoded;
use util::generate_hex_key;

#[derive(Debug)]
enum AccountError {
    TokenNotFound,
    CouldNotParseToken,
    InvalidResponse(String),
}

#[derive(Serialize, Deserialize)]
struct AccessTokenResponse {
    access_token: String,
    timestamp: String,
    expiration: String,
}

#[derive(Serialize, Deserialize, Default, Debug)]
struct Settings {
    secret_key: Option<String>, // used for encryption
                                // other settings...
}

impl std::fmt::Display for AccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenNotFound => write!(f, "Access token not found in response"),
            Self::CouldNotParseToken => write!(f, "Could not parse access token"),
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

/// Launches RotMG.exe
#[tauri::command]
async fn launch_exalt(
    exalt_path: &str,
    device_token: &str,
    guid: &str,
    password: &str,
) -> Result<String, String> {
    // Build and verify the path to the executable
    let mut exalt_path = PathBuf::from(exalt_path);
    exalt_path.push("RotMG Exalt.exe");

    if !exalt_path.exists() {
        let err = format!("Exalt executable not found at: {:?}", exalt_path);
        log::error!("[Exalt] {}", err);
        return Err(err);
    }

    log::info!("[Exalt] Launching Exalt at: {:?}", exalt_path);

    // Get access token
    let access_token: AccessTokenResponse = get_access_token(guid, password, Some(device_token))
        .await
        .map_err(|e| {
            log::error!("[Exalt] Failed to get access token: {}", e);
            e.to_string()
        })?;

    // Encode parameters in Base64
    let encoded_guid = general_purpose::STANDARD.encode(guid);
    let encoded_token = general_purpose::STANDARD.encode(&access_token.access_token);
    let encoded_timestamp = general_purpose::STANDARD.encode(&access_token.timestamp);
    let encoded_expiration = general_purpose::STANDARD.encode(&access_token.expiration);

    // Build the arguments string
    let args = format!(
        "data:{{platform:Deca,guid:{},token:{},tokenTimestamp:{},tokenExpiration:{},env:4}}",
        encoded_guid, encoded_token, encoded_timestamp, encoded_expiration
    );

    log::info!("[Exalt] Launching...");

    // Start the process with explicit working directory
    match Command::new(&exalt_path)
        .current_dir(exalt_path.parent().ok_or("Invalid path")?)
        .arg(&args)
        .spawn()
    {
        Ok(_) => {
            log::info!("[Exalt] Successfully launched");
            Ok("Successfully launched Exalt".to_string())
        }
        Err(e) => {
            let err = format!("Failed to launch Exalt: {}", e);
            log::error!("[Exalt] {}", err);
            Err(err)
        }
    }
}

/// Gets an access token from the RotMG API.
/// This is used to authenticate the user and get the data from the API.
async fn get_access_token(
    guid: &str,
    password: &str,
    device_token: Option<&str>,
) -> Result<AccessTokenResponse, String> {
    let device_token = device_token.unwrap_or("0");

    let base_url: &str = "https://www.realmofthemadgod.com";
    let client: Client = Client::new();

    let is_steam: bool = guid.starts_with("steamworks:");

    let mut form_data: Vec<(&str, &str)> = vec![("clientToken", device_token), ("guid", guid)];

    if is_steam {
        form_data.push(("steamid", guid));
        form_data.push(("secret", password));
    } else {
        form_data.push(("password", password));
    }

    let verify_url: String = format!("{}/account/verify", base_url);
    log::info!("[RotMG API] Sending /account/verify request");

    let response: reqwest::Response = client
        .post(&verify_url)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&form_data)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: String = response.text().await.map_err(|e| e.to_string())?;

    log::info!("[RotMG API] Parsing Access Token");

    // Create regex patterns with named capture groups.
    let token_regex = Regex::new(r#"<AccessToken>(?P<access_token>.*?)</AccessToken>"#)
        .map_err(|e| AccountError::InvalidResponse(e.to_string()).to_string())?;
    let ts_regex = Regex::new(r#"<AccessTokenTimestamp>(?P<timestamp>.*?)</AccessTokenTimestamp>"#)
        .map_err(|e| AccountError::InvalidResponse(e.to_string()).to_string())?;
    let exp_regex =
        Regex::new(r#"<AccessTokenExpiration>(?P<expiration>.*?)</AccessTokenExpiration>"#)
            .map_err(|e| AccountError::InvalidResponse(e.to_string()).to_string())?;

    // Capture access token.
    let token_caps = token_regex.captures(&body).ok_or_else(|| {
        log::error!(
            "[RotMG API] Access token not found. Response body: {}",
            body
        );
        AccountError::TokenNotFound.to_string()
    })?;

    let access_token = token_caps
        .name("access_token")
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| {
            log::error!(
                "[RotMG API] Failed to parse access token. Response body: {}",
                body
            );
            AccountError::CouldNotParseToken.to_string()
        })?;

    let ts_caps = ts_regex
        .captures(&body)
        .ok_or_else(|| "Access token timestamp not found".to_string())?;

    let timestamp = ts_caps
        .name("timestamp")
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Could not parse access token timestamp".to_string())?;

    // Capture expiration.
    let exp_caps = exp_regex
        .captures(&body)
        .ok_or_else(|| "Access token expiration not found".to_string())?;

    let expiration = exp_caps
        .name("expiration")
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| "Could not parse access token expiration".to_string())?;

    // populate model
    let access_token_response = AccessTokenResponse {
        access_token,
        timestamp,
        expiration,
    };

    Ok(access_token_response)
}

/// Gets the account data (charlist) from the RotMG API.
// credits: https://github.com/faynt0
#[tauri::command]
async fn get_account_data(guid: &str, password: &str) -> Result<String, String> {
    let base_url: &str = "https://www.realmofthemadgod.com";
    let client: Client = Client::new();

    let start = Instant::now();

    // get access token
    let access_token: AccessTokenResponse = get_access_token(guid, password, None)
        .await
        .map_err(|e| e.to_string())?;

    // send another request to /charlist
    let char_list_url: String = format!(
        "{}/char/list?muleDump=true&{}",
        base_url,
        form_urlencoded::Serializer::new(String::new())
            .append_pair("accessToken", &access_token.access_token)
            .finish()
    );

    log::info!("[RotMG API] Sending /char/list request");

    let char_list_response = client
        .get(char_list_url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let char_list_body = char_list_response.text().await.map_err(|e| e.to_string())?;

    let duration = start.elapsed();

    log::info!(
        "[RotMG API] Request completed in {:.2}ms",
        duration.as_secs_f64() * 1000.0
    );

    Ok(char_list_body)
}

/// Gets the settings from the settings file.
/// If the file does not exist, it will generate a new one.
#[tauri::command]
async fn get_settings() -> Result<Settings, String> {
    let settings_path: PathBuf = get_settings_file_path();
    log::info!("[Settings] Checking settings at: {:?}", settings_path);

    let settings: Settings = if PathBuf::from(&settings_path).exists() {
        let content: String = fs::read_to_string(&settings_path).map_err(|e| e.to_string())?;
        log::info!("[Settings] Found existing settings");
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        log::info!("[Settings] No settings found, generating settings file.");
        // generate new key
        let key: String = generate_hex_key(32);

        let new_settings = Settings {
            secret_key: Some(key),
            ..Default::default()
        };

        // save new settings
        let settings_json =
            serde_json::to_string_pretty(&new_settings).map_err(|e| e.to_string())?;
        fs::write(&settings_path, settings_json).map_err(|e| e.to_string())?;
        log::info!("[Settings] New settings file created at: {:?}", settings_path);
        

        new_settings
    };

    log::info!(
            "[Settings] New settings: {}",
            serde_json::to_string_pretty(&settings).unwrap_or_default()
        );

    Ok(settings)
}

/// Gets the path to the save file directory.
fn get_save_file_path() -> PathBuf {
    #[cfg(target_os = "android")]
    let path: PathBuf = {
        // Path for Android: /data/data/com.rotmg_stash.app/files/RotMG Stash
        // Replace "com.rotmg_stash.app" with your actual application ID if it's different.
        let base_dir = PathBuf::from("/data/data/com.rotmg_stash.app/files");

        if !base_dir.exists() {
            log::info!("[Settings] Creating directory for Android: {:?}", base_dir);
            if let Err(e) = fs::create_dir_all(&base_dir) {
                log::error!("Failed to create directory on Android: {}. Using current dir as fallback.", e);
                // Fallback to current directory or handle error more gracefully
                return PathBuf::from(".");
            }
        }
        base_dir
    };

    #[cfg(not(target_os = "android"))]
    let path: PathBuf = {
        let local_data_dir = dirs::data_local_dir()
            .ok_or("Could not get local data directory")
            .unwrap(); // Consider handling this error more gracefully than unwrap
        let app_data_path = local_data_dir.join("RotMG Stash");

        if !app_data_path.exists() {
            log::info!("[Settings] Creating directory: {:?}", app_data_path);
            fs::create_dir_all(&app_data_path)
                .map_err(|e| format!("Failed to create directory: {}", e))
                .unwrap(); // Consider handling this error more gracefully than unwrap
        }
        app_data_path
    };

    path
}

/// Gets the path to the settings file.
fn get_settings_file_path() -> PathBuf {
    let base_path: PathBuf = get_save_file_path();
    log::info!("[Settings] Base path: {:?}", base_path);

    let settings_path: PathBuf = PathBuf::from(base_path).join("rotmg-stash-settings.json");
    log::info!("[Settings] Settings path: {:?}", settings_path);
    settings_path
}

// Add this near your other command functions
#[tauri::command]
async fn execute_powershell() -> Result<String, String> {
    log::info!("[PowerShell] Executing script");

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            devicetoken_script::DEVICE_TOKEN_POWERSHELLSCRIPT,
        ])
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map_err(|e| format!("Failed to parse PowerShell output: {}", e))?;
        Ok(stdout)
    } else {
        let stderr = String::from_utf8(output.stderr)
            .map_err(|e| format!("Failed to parse PowerShell error: {}", e))?;
        Err(format!("PowerShell script failed: {}", stderr))
    }
}

// Update your invoke_handler to include the new command
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir { file_name: None },
                ))
                .max_file_size(100000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_account_data,
            get_settings,
            launch_exalt,
            execute_powershell
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
