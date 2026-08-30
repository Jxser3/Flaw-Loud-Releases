
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, sync::Mutex};

const DEV_PASSWORD_HASH: &str = "b8667045155bfceef59084f123f2a9cfaae9f0117056a61e4021a84e9415e6b1";
const RC_TOKEN_SALT: &str = "flaw-loud-rc-v0922";

fn app_data_dir() -> PathBuf {
    let base = env::var("LOCALAPPDATA").unwrap_or_else(|_| env::temp_dir().to_string_lossy().to_string());
    let dir = PathBuf::from(base).join("Flaw Loud");
    let _ = fs::create_dir_all(&dir);
    dir
}
fn rc_token_path() -> PathBuf { app_data_dir().join("rc_access.token") }
fn saved_license_path() -> PathBuf { app_data_dir().join("license.cache") }
fn rc_token_value() -> String { format!("{:x}", Sha256::digest(format!("{}|{}|{}", RC_TOKEN_SALT, hwid(), DEV_PASSWORD_HASH).as_bytes())) }
fn has_valid_rc_token() -> bool { fs::read_to_string(rc_token_path()).ok().map(|v| v.trim()==rc_token_value()).unwrap_or(false) }
fn set_rc_token(enabled: bool) { if enabled { let _=fs::write(rc_token_path(), rc_token_value()); } else { let _=fs::remove_file(rc_token_path()); } }
fn protect_secret(secret: &str) -> String {
    let key=Sha256::digest(format!("flaw-license|{}", hwid()).as_bytes());
    secret.as_bytes().iter().enumerate().map(|(i,b)| format!("{:02x}", b ^ key[i%key.len()])).collect()
}
fn unprotect_secret(encoded: &str) -> Option<String> {
    if encoded.len()%2!=0 { return None; }
    let key=Sha256::digest(format!("flaw-license|{}", hwid()).as_bytes());
    let mut out=Vec::new();
    for i in (0..encoded.len()).step_by(2) {
        let byte=u8::from_str_radix(&encoded[i..i+2],16).ok()?;
        out.push(byte ^ key[(i/2)%key.len()]);
    }
    String::from_utf8(out).ok()
}
fn save_license_key(key: &str) { let _=fs::write(saved_license_path(), protect_secret(key)); }
fn load_license_key() -> Option<String> { fs::read_to_string(saved_license_path()).ok().and_then(|v| unprotect_secret(v.trim())) }
fn clear_saved_license() { let _=fs::remove_file(saved_license_path()); }

#[derive(Clone, Deserialize)]
pub struct LicenseConfig {
    pub enabled: bool,
    pub dev_bypass: bool,
    pub name: String,
    pub owner_id: String,
    pub version: String,
    pub api_url: String,
}
impl Default for LicenseConfig {
    fn default() -> Self {
        serde_json::from_str(include_str!("../keyauth.json")).unwrap_or_else(|_| Self {
            enabled: false,
            dev_bypass: true,
            name: "Flaw Loud".into(),
            owner_id: "PASTE_OWNER_ID".into(),
            version: "0.9.5-rc.1".into(),
            api_url: "https://keyauth.win/api/1.3/".into(),
        })
    }
}

#[derive(Clone, Serialize, Default)]
pub struct LicenseStatus {
    pub configured: bool,
    pub required: bool,
    pub authenticated: bool,
    pub dev_bypass_available: bool,
    pub username: String,
    pub subscription: String,
    pub expiry: String,
    pub message: String,
}

#[derive(Default)]
struct RuntimeLicense {
    authenticated: bool,
    username: String,
    subscription: String,
    expiry: String,
    session_id: String,
}

pub struct LicenseState {
    config: LicenseConfig,
    runtime: Mutex<RuntimeLicense>,
}
impl Default for LicenseState {
    fn default() -> Self {
        Self { config: LicenseConfig::default(), runtime: Mutex::new(RuntimeLicense::default()) }
    }
}

#[derive(Deserialize)]
struct ApiSubscription {
    #[serde(default)] subscription: String,
    #[serde(default)] expiry: String,
}
#[derive(Deserialize, Default)]
struct ApiInfo {
    #[serde(default)] username: String,
    #[serde(default)] subscriptions: Vec<ApiSubscription>,
}
#[derive(Deserialize)]
struct ApiResponse {
    success: bool,
    #[serde(default)] message: String,
    #[serde(default)] ownerid: String,
    #[serde(default)] sessionid: String,
    #[serde(default)] info: ApiInfo,
}

fn configured(c: &LicenseConfig) -> bool {
    c.enabled && !c.name.trim().is_empty() && c.owner_id.len() == 10 && c.owner_id != "PASTE_OWNER_ID"
}
fn hwid() -> String {
    let raw = format!("{}|{}|{}|{}",
        env::var("COMPUTERNAME").unwrap_or_default(),
        env::var("USERNAME").unwrap_or_default(),
        env::var("PROCESSOR_IDENTIFIER").unwrap_or_default(),
        env::var("SystemDrive").unwrap_or_default()
    );
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}
fn executable_hash() -> String {
    let bytes = env::current_exe().ok().and_then(|p| fs::read(p).ok()).unwrap_or_default();
    format!("{:x}", Sha256::digest(&bytes))
}
async fn get_json(client: &reqwest::Client, url: &str, query: &[(&str, String)]) -> Result<ApiResponse, String> {
    let res = client.get(url).query(query).send().await.map_err(|e| format!("License network error: {e}"))?;
    let status = res.status();
    let text = res.text().await.map_err(|e| format!("License response error: {e}"))?;
    if !status.is_success() { return Err(format!("License server returned HTTP {status}")); }
    serde_json::from_str::<ApiResponse>(&text).map_err(|_| format!("Unexpected license response: {}", text.chars().take(120).collect::<String>()))
}
async fn init_session(c: &LicenseConfig) -> Result<String, String> {
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e|e.to_string())?;
    let q = [
        ("type", "init".to_string()),
        ("ver", c.version.clone()),
        ("hash", executable_hash()),
        ("name", c.name.clone()),
        ("ownerid", c.owner_id.clone()),
    ];
    let r = get_json(&client, &c.api_url, &q).await?;
    if !r.ownerid.is_empty() && r.ownerid != c.owner_id { return Err("License owner verification failed.".into()); }
    if !r.success { return Err(if r.message.is_empty() {"KeyAuth initialization failed.".into()} else {r.message}); }
    if r.sessionid.is_empty() { return Err("License service did not return a session.".into()); }
    Ok(r.sessionid)
}

#[tauri::command]
pub fn get_license_status(state: tauri::State<'_, LicenseState>) -> Result<LicenseStatus, String> {
    let c=&state.config;
    let mut r=state.runtime.lock().map_err(|_|"License state lock failed")?;
    if !r.authenticated && c.dev_bypass && has_valid_rc_token() {
        r.authenticated=true;
        r.username="RC Preview".into();
        r.subscription="Developer".into();
    }
    Ok(LicenseStatus{
        configured: configured(c),
        required: configured(c),
        authenticated: r.authenticated,
        dev_bypass_available: c.dev_bypass,
        username: r.username.clone(),
        subscription: r.subscription.clone(),
        expiry: r.expiry.clone(),
        message: if configured(c) {
            if r.authenticated {"License active".into()} else {"Activation required".into()}
        } else if c.dev_bypass {"RC developer preview enabled".into()} else {"KeyAuth is not configured".into()}
    })
}

#[tauri::command]
pub async fn activate_license(key: String, remember: Option<bool>, state: tauri::State<'_, LicenseState>) -> Result<LicenseStatus, String> {
    let c=state.config.clone();
    if !configured(&c) { return Err("KeyAuth is not configured yet. Edit src-tauri/keyauth.json first.".into()); }
    let key=key.trim().to_string();
    if key.len()<4 { return Err("Enter a valid license key.".into()); }
    let session=init_session(&c).await?;
    let client=reqwest::Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e|e.to_string())?;
    let q=[
        ("type","license".to_string()),
        ("key",key.clone()),
        ("hwid",hwid()),
        ("sessionid",session.clone()),
        ("name",c.name.clone()),
        ("ownerid",c.owner_id.clone()),
        ("code","".to_string()),
    ];
    let response=get_json(&client,&c.api_url,&q).await?;
    if !response.ownerid.is_empty() && response.ownerid != c.owner_id { return Err("License owner verification failed.".into()); }
    if !response.success { return Err(if response.message.is_empty() {"License rejected.".into()} else {response.message}); }
    let sub=response.info.subscriptions.first();
    let mut r=state.runtime.lock().map_err(|_|"License state lock failed")?;
    r.authenticated=true;
    r.username=response.info.username;
    r.subscription=sub.map(|s|s.subscription.clone()).unwrap_or_else(||"Licensed".into());
    r.expiry=sub.map(|s|s.expiry.clone()).unwrap_or_default();
    r.session_id=session;
    drop(r);
    if remember.unwrap_or(false) { save_license_key(&key); } else { clear_saved_license(); }
    get_license_status(state)
}

#[tauri::command]
pub fn use_developer_preview(password: String, remember: Option<bool>, state: tauri::State<'_, LicenseState>) -> Result<LicenseStatus, String> {
    if !state.config.dev_bypass { return Err("Developer preview is disabled.".into()); }
    let supplied=format!("{:x}", Sha256::digest(password.as_bytes()));
    if supplied != DEV_PASSWORD_HASH { return Err("Incorrect RC developer password.".into()); }
    let mut r=state.runtime.lock().map_err(|_|"License state lock failed")?;
    r.authenticated=true;
    r.username="RC Preview".into();
    r.subscription="Developer".into();
    drop(r);
    set_rc_token(remember.unwrap_or(false));
    get_license_status(state)
}

#[tauri::command]
pub async fn restore_saved_license(state: tauri::State<'_, LicenseState>) -> Result<LicenseStatus, String> {
    let key=load_license_key().ok_or_else(||"No remembered license on this PC.".to_string())?;
    activate_license(key, Some(true), state).await
}

#[tauri::command]
pub fn logout_license(state: tauri::State<'_, LicenseState>) -> Result<LicenseStatus, String> {
    set_rc_token(false);
    clear_saved_license();
    let mut r=state.runtime.lock().map_err(|_|"License state lock failed")?;
    *r=RuntimeLicense::default();
    drop(r);
    get_license_status(state)
}
