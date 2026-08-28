use crate::config::{CONFIG, StrValue};
use ed25519_dalek::SigningKey;
use sunset::{KeyType, SignKey};

const SSH_KEY_CONFIG: &str = "ssh_key";

/// Loads the ed25519 signing key stored in flash, if one has been generated.
pub async fn load_signing_key() -> Option<SigningKey> {
    let mut config = CONFIG.get().lock().await;
    let stored = config.fetch(SSH_KEY_CONFIG).await.ok()??;
    let seed = hex_to_bytes(stored.as_str())?;
    Some(SigningKey::from_bytes(&seed))
}

async fn store_signing_key(key: &SigningKey) -> Result<(), ()> {
    let hex = bytes_to_hex(&key.to_bytes());
    let value: StrValue = hex.as_str().try_into().map_err(|_| ())?;
    let mut config = CONFIG.get().lock().await;
    config.store(SSH_KEY_CONFIG, value).await.map_err(|_| ())
}

async fn print_public_key(key: &SigningKey) {
    let blob = pubkey_blob(&key.verifying_key().to_bytes());
    let encoded = base64_encode(&blob);
    // Also log it, so it can be copied exactly from a serial terminal
    // (USB-CDC or the debug UART) instead of hand-transcribed off the LCD.
    log::info!("ssh-ed25519 {encoded} picocalc-ssh-client");
    print!("ssh-ed25519 {encoded} picocalc-ssh-client\r\n");
}

/// Handles the local `keygen` shell command: generates and stores a new
/// on-device Ed25519 keypair (the private key never leaves the device),
/// or re-displays the public key of an already generated one.
pub async fn keygen_command(args: &[&str]) {
    match args {
        ["keygen"] | ["keygen", "force"] => {
            let force = args.len() == 2;
            if !force && load_signing_key().await.is_some() {
                print!(
                    "A key already exists. Use `keygen force` to replace it \
                     (servers you've already authorized will stop accepting it).\r\n"
                );
                return;
            }
            let generated = match SignKey::generate(KeyType::Ed25519, None) {
                Ok(k) => k,
                Err(err) => {
                    print!("keygen failed: {err:?}\r\n");
                    return;
                }
            };
            let key = match &generated {
                SignKey::Ed25519(k) => k.clone(),
                _ => unreachable!(),
            };
            if store_signing_key(&key).await.is_err() {
                print!("failed to store key\r\n");
                return;
            }
            print!(
                "New SSH key generated. Add this line to the server's \
                 ~/.ssh/authorized_keys:\r\n\r\n"
            );
            print_public_key(&key).await;
        }
        ["keygen", "show"] => match load_signing_key().await {
            Some(key) => print_public_key(&key).await,
            None => print!("No SSH key configured. Run `keygen` to create one.\r\n"),
        },
        _ => {
            print!("Usage: keygen [force|show]\r\n");
        }
    }
}

/// SSH wire-format public key blob: string("ssh-ed25519") + string(pubkey).
fn pubkey_blob(pubkey: &[u8; 32]) -> [u8; 51] {
    let mut blob = [0u8; 51];
    let name = b"ssh-ed25519";
    blob[0..4].copy_from_slice(&(name.len() as u32).to_be_bytes());
    blob[4..15].copy_from_slice(name);
    blob[15..19].copy_from_slice(&(pubkey.len() as u32).to_be_bytes());
    blob[19..51].copy_from_slice(pubkey);
    blob
}

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> heapless::String<96> {
    let mut out = heapless::String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64_ALPHABET[((n >> 18) & 0x3f) as usize] as char)
            .ok();
        out.push(B64_ALPHABET[((n >> 12) & 0x3f) as usize] as char)
            .ok();
        out.push(if chunk.len() > 1 {
            B64_ALPHABET[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        })
        .ok();
        out.push(if chunk.len() > 2 {
            B64_ALPHABET[(n & 0x3f) as usize] as char
        } else {
            '='
        })
        .ok();
    }
    out
}

const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

fn bytes_to_hex(bytes: &[u8; 32]) -> heapless::String<64> {
    let mut out = heapless::String::new();
    for b in bytes {
        out.push(HEX_CHARS[(b >> 4) as usize] as char).ok();
        out.push(HEX_CHARS[(b & 0x0f) as usize] as char).ok();
    }
    out
}

fn hex_to_bytes(s: &str) -> Option<[u8; 32]> {
    let bytes = s.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in bytes.chunks(2).enumerate() {
        out[i] = (hex_val(pair[0])? << 4) | hex_val(pair[1])?;
    }
    Some(out)
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
