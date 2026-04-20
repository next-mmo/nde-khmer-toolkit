use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use sha2::{Digest, Sha256};

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const WINDOWS_EPOCH_SECONDS: u64 = 11_644_473_600;
const HUNDRED_NANOSECONDS_PER_SECOND: u64 = 10_000_000;

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime::now panic")
        .as_millis()
}

pub fn gen_request_id() -> String {
    Uuid::new_v4().to_string().replace("-", "")
}

pub fn gen_muid() -> String {
    gen_request_id().to_uppercase()
}

pub fn gen_sec_ms_gec() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime::now panic")
        .as_secs();
    let mut windows_seconds = now + WINDOWS_EPOCH_SECONDS;
    windows_seconds -= windows_seconds % 300;

    let ticks = windows_seconds * HUNDRED_NANOSECONDS_PER_SECOND;
    let token = format!("{ticks}{TRUSTED_CLIENT_TOKEN}");
    let digest = Sha256::digest(token.as_bytes());

    format!("{digest:X}")
}
