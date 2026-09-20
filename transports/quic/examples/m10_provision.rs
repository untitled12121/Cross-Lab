//! Generates throwaway M10 Linux/Android development provisioning files.
//!
//! The generated authority and device secrets are synthetic and must never be
//! reused as production Cross-Lab identity material.

use std::{
    env, fs,
    io::Write as _,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use crosslab_crypto::{SigningKey, random_bytes};
use serde_json::{Value, json};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse()?;
    let remote: SocketAddr = args
        .server_remote
        .parse()
        .map_err(|_| "--server-remote must be a socket address such as 192.0.2.10:45777")?;
    if remote.ip().is_unspecified() || remote.port() == 0 {
        return Err("--server-remote must use a concrete address and non-zero port".into());
    }

    prepare_output_dir(&args.out_dir)?;

    let owner_id = random()?;
    let owner_root_secret = random()?;
    let device_signing_secret = random()?;
    let desktop_device_id = random()?;
    let desktop_device_secret = random()?;
    let android_device_id = random()?;
    let android_device_secret = random()?;

    let desktop_public_key = SigningKey::from_secret_bytes(desktop_device_secret)
        .verifying_key()
        .to_bytes();
    let android_public_key = SigningKey::from_secret_bytes(android_device_secret)
        .verifying_key()
        .to_bytes();

    let certified = rcgen::generate_simple_self_signed(vec![args.server_name.clone()])
        .map_err(|_| "failed to generate development TLS certificate")?;
    let certificate_der = certified.cert.der().as_ref().to_vec();
    let private_key_der = certified.signing_key.serialize_der();

    let server_bind = SocketAddr::new(
        match remote.ip() {
            IpAddr::V4(_) => "0.0.0.0".parse().expect("valid IPv4 wildcard"),
            IpAddr::V6(_) => "::".parse().expect("valid IPv6 wildcard"),
        },
        remote.port(),
    );

    let desktop = provisioning_document(
        SharedIdentity {
            owner_id,
            owner_root_secret,
            device_signing_secret,
        },
        DeviceIdentity {
            local_device_id: desktop_device_id,
            local_device_secret: desktop_device_secret,
            peer_device_id: android_device_id,
            peer_public_key: android_public_key,
        },
        json!({
            "role": "server",
            "bind_addr": server_bind.to_string(),
            "certificate_chain_der_hex": [hex(&certificate_der)],
            "private_key_pkcs8_der_hex": hex(&private_key_der),
        }),
    );
    let android = provisioning_document(
        SharedIdentity {
            owner_id,
            owner_root_secret,
            device_signing_secret,
        },
        DeviceIdentity {
            local_device_id: android_device_id,
            local_device_secret: android_device_secret,
            peer_device_id: desktop_device_id,
            peer_public_key: desktop_public_key,
        },
        json!({
            "role": "client",
            "bind_addr": match remote.ip() {
                IpAddr::V4(_) => "0.0.0.0:0",
                IpAddr::V6(_) => "[::]:0",
            },
            "remote_addr": remote.to_string(),
            "server_name": args.server_name,
            "trusted_server_certificate_der_hex": [hex(&certificate_der)],
        }),
    );

    let desktop_path = args.out_dir.join("desktop.json");
    let android_path = args.out_dir.join("android.json");
    write_private(
        &desktop_path,
        &serde_json::to_vec_pretty(&desktop).map_err(json_error)?,
    )?;
    write_private(
        &android_path,
        &serde_json::to_vec_pretty(&android).map_err(json_error)?,
    )?;

    println!("created {}", desktop_path.display());
    println!("created {}", android_path.display());
    println!("development-only synthetic credentials; delete both files after evidence collection");
    Ok(())
}

#[derive(Clone, Copy)]
struct SharedIdentity {
    owner_id: [u8; 32],
    owner_root_secret: [u8; 32],
    device_signing_secret: [u8; 32],
}

#[derive(Clone, Copy)]
struct DeviceIdentity {
    local_device_id: [u8; 32],
    local_device_secret: [u8; 32],
    peer_device_id: [u8; 32],
    peer_public_key: [u8; 32],
}

fn provisioning_document(shared: SharedIdentity, device: DeviceIdentity, endpoint: Value) -> Value {
    json!({
        "identity": {
            "owner_id_hex": hex(&shared.owner_id),
            "owner_root_secret_hex": hex(&shared.owner_root_secret),
            "owner_root_epoch": 0,
            "device_signing_secret_hex": hex(&shared.device_signing_secret),
            "device_signing_epoch": 0,
            "local_device_id_hex": hex(&device.local_device_id),
            "local_device_secret_hex": hex(&device.local_device_secret),
            "local_credential_epoch": 0,
            "peer_device_id_hex": hex(&device.peer_device_id),
            "peer_device_public_key_hex": hex(&device.peer_public_key),
            "peer_credential_epoch": 0
        },
        "protocol": {
            "ranges": [{
                "major": 1,
                "min_minor": 0,
                "max_minor": 2
            }],
            "supported_features": [2],
            "required_features": []
        },
        "endpoint": endpoint
    })
}

struct Args {
    server_remote: String,
    server_name: String,
    out_dir: PathBuf,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut server_remote = None;
        let mut server_name = "crosslab.local".to_owned();
        let mut out_dir = None;
        let mut args = env::args().skip(1);

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--server-remote" => {
                    server_remote = Some(next_value(&mut args, "--server-remote")?);
                }
                "--server-name" => {
                    server_name = next_value(&mut args, "--server-name")?;
                }
                "--out-dir" => {
                    out_dir = Some(PathBuf::from(next_value(&mut args, "--out-dir")?));
                }
                "--help" | "-h" => {
                    println!(
                        "usage: m10_provision --server-remote <LAN_IP:PORT> --out-dir <ABSOLUTE_DIR> [--server-name crosslab.local]"
                    );
                    std::process::exit(0);
                }
                _ => return Err(format!("unknown argument: {argument}")),
            }
        }

        let server_remote =
            server_remote.ok_or_else(|| "missing required --server-remote".to_owned())?;
        let out_dir = out_dir.ok_or_else(|| "missing required --out-dir".to_owned())?;
        if !out_dir.is_absolute() {
            return Err(
                "--out-dir must be absolute so secrets are not created in the repository".into(),
            );
        }
        if server_name.is_empty() {
            return Err("--server-name must not be empty".into());
        }

        Ok(Self {
            server_remote,
            server_name,
            out_dir,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {name}"))
}

fn random<const N: usize>() -> Result<[u8; N], String> {
    random_bytes::<N>().map_err(|_| "secure random generation failed".into())
}

fn prepare_output_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|_| "failed to create output directory")?;
    set_private_dir_permissions(path)
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "failed to restrict output directory permissions".into())
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt as _};

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| "refusing to overwrite provisioning file")?;
    file.write_all(bytes)
        .map_err(|_| "failed to write provisioning file".to_owned())
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "refusing to overwrite provisioning file")?;
    file.write_all(bytes)
        .map_err(|_| "failed to write provisioning file".to_owned())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn json_error(_: serde_json::Error) -> String {
    "failed to encode provisioning document".into()
}
