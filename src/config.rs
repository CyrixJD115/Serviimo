//! Shared JSON configuration and command-line overrides for all services.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub bind_addresses: BindAddresses,
    pub advertised_gate_addr: Option<String>,
    pub rsa_keypair_path: PathBuf,
    pub save_path: PathBuf,
    pub cdn_document_root: PathBuf,
    pub aniisignal_config_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BindAddresses {
    pub gate: String,
    pub cdn: String,
    pub office: String,
    pub world: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addresses: BindAddresses::default(),
            advertised_gate_addr: None,
            rsa_keypair_path: PathBuf::from("serviimo-data/gate-key.pem"),
            save_path: PathBuf::from("serviimo-data/serviimo.sqlite3"),
            cdn_document_root: PathBuf::from("cdn-root"),
            aniisignal_config_path: PathBuf::from("AniiSignal/AniiSignal.json"),
        }
    }
}

impl Default for BindAddresses {
    fn default() -> Self {
        Self {
            gate: "127.0.0.1:10000".to_string(),
            cdn: "127.0.0.1:18080".to_string(),
            office: "127.0.0.1:18081".to_string(),
            world: "127.0.0.1:20000".to_string(),
        }
    }
}

impl Config {
    pub fn from_args(args: &[String]) -> Result<Option<Self>, Box<dyn Error>> {
        let mut config_path = PathBuf::from("serviimo.json");
        let mut explicit_config = false;
        let mut overrides: Vec<(String, String)> = Vec::new();
        let mut index = 0;

        while index < args.len() {
            let argument = &args[index];
            if argument == "--help" || argument == "-h" {
                print_usage();
                return Ok(None);
            }
            let (name, value) = if let Some((name, value)) = argument.split_once('=') {
                (name.to_string(), Some(value.to_string()))
            } else {
                (argument.clone(), None)
            };
            if name == "--config" {
                let value = option_value(&args, &mut index, value, &name)?;
                config_path = PathBuf::from(value);
                explicit_config = true;
            } else {
                let value = option_value(&args, &mut index, value, &name)?;
                overrides.push((name, value));
            }
            index += 1;
        }

        let mut config = if config_path.exists() {
            let bytes = fs::read(&config_path)?;
            serde_json::from_slice::<Config>(&bytes)?
        } else if explicit_config {
            return Err(format!("configuration file not found: {}", config_path.display()).into());
        } else {
            Config::default()
        };
        for (name, value) in overrides {
            match name.as_str() {
                "--gate-bind" => config.bind_addresses.gate = value,
                "--cdn-bind" => config.bind_addresses.cdn = value,
                "--office-bind" => config.bind_addresses.office = value,
                "--world-bind" => config.bind_addresses.world = value,
                "--public-gate" => config.advertised_gate_addr = Some(value),
                "--keypair-path" => config.rsa_keypair_path = PathBuf::from(value),
                "--save-path" => config.save_path = PathBuf::from(value),
                "--cdn-root" => config.cdn_document_root = PathBuf::from(value),
                "--aniisignal-config" => config.aniisignal_config_path = PathBuf::from(value),
                _ => return Err(format!("unknown option: {}", name).into()),
            }
        }
        Ok(Some(config))
    }

    pub fn published_gate_address(&self) -> &str {
        self.advertised_gate_addr
            .as_deref()
            .unwrap_or(&self.bind_addresses.gate)
    }

    pub fn write_aniisignal_config(&self, public_key_pem: &str) -> Result<(), Box<dyn Error>> {
        if let Some(parent) = self
            .aniisignal_config_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let document = json!({
            "CdnBaseUrl": format!("http://{}", self.bind_addresses.cdn),
            "OfficeBaseUrl": format!("http://{}", self.bind_addresses.office),
            "GatePublicKeyPem": public_key_pem,
            "InjectGateKey": true,
            "RedirectEndpoints": true,
            "StubAntiCheat": false,
            "BlockOutboundTelemetry": false
        });
        fs::write(
            &self.aniisignal_config_path,
            serde_json::to_vec_pretty(&document)?,
        )?;
        Ok(())
    }
}

fn option_value(
    args: &[String],
    index: &mut usize,
    inline_value: Option<String>,
    name: &str,
) -> Result<String, Box<dyn Error>> {
    if let Some(value) = inline_value {
        return Ok(value);
    }
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("missing value for {}", name).into())
}

fn print_usage() {
    println!(
        "Usage: serviimo [--config PATH] [options]\n\
         Options:\n\
           --gate-bind ADDR       TCP login gate bind address\n\
           --cdn-bind ADDR        CDN HTTP bind address\n\
           --office-bind ADDR     office HTTP bind address\n\
           --world-bind ADDR      reserved world gate bind address\n\
           --public-gate ADDR     gate address published to the client\n\
           --keypair-path PATH    RSA private key path\n\
           --save-path PATH       SQLite database path\n\
           --cdn-root PATH        CDN document root\n\
           --aniisignal-config PATH  generated AniiSignal JSON path"
    );
}

pub fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}
