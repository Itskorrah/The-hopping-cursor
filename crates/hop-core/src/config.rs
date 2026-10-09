use crate::model::{HopKey, LayoutDisplay, Settings, TrustedPeer};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

pub const NOISE: &str = "Noise_XX_25519_ChaChaPoly_SHA256";
#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub private_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_identity: Option<ProtectedIdentity>,
    pub settings: Settings,
    pub peers: BTreeMap<String, TrustedPeer>,
    pub layout: Vec<LayoutDisplay>,
    #[serde(default)]
    pub profiles: BTreeMap<String, Vec<LayoutDisplay>>,
    #[serde(default)]
    pub hop_keys: Vec<HopKey>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct ProtectedIdentity {
    pub kind: String,
    pub reference: String,
    pub public_key: String,
}
pub struct Identity {
    pub private: Zeroizing<Vec<u8>>,
    pub public: Vec<u8>,
    pub id: String,
}
impl Identity {
    pub fn from_config(config: &Config) -> Result<Self> {
        let private =
            Zeroizing::new(hex::decode(&config.private_key).context("Invalid private identity")?);
        anyhow::ensure!(private.len() == 32, "Invalid private identity length");
        // Snow's resolver derives the public key through a static-key handshake.
        let resolver = snow::resolvers::DefaultResolver;
        use snow::resolvers::CryptoResolver;
        let mut dh = resolver
            .resolve_dh(&snow::params::DHChoice::Curve25519)
            .context("X25519 unavailable")?;
        dh.set(&private);
        let public = dh.pubkey().to_vec();
        let id = device_id(&public);
        Ok(Self {
            private,
            public,
            id,
        })
    }
}
pub fn device_id(key: &[u8]) -> String {
    hex::encode(Sha256::digest(key))[..32].into()
}
pub fn fingerprint(key: &[u8]) -> String {
    let h = hex::encode(Sha256::digest(key));
    h.as_bytes()
        .chunks(4)
        .map(|x| std::str::from_utf8(x).unwrap())
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn safety_code(a: &[u8], b: &[u8]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"the-hopping-cursor/pair/v1");
    if a < b {
        hash.update(a);
        hash.update(b);
    } else {
        hash.update(b);
        hash.update(a);
    }
    fingerprint(&hash.finalize())
}
pub fn default_config_dir() -> Result<PathBuf> {
    Ok(
        directories::ProjectDirs::from("com", "Itskorrah", "TheHoppingCursor")
            .context("No application data directory")?
            .config_dir()
            .to_owned(),
    )
}
pub fn load(dir: &Path) -> Result<Config> {
    fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
    }
    let path = dir.join("config.json");
    if path.exists() {
        let mut config: Config = serde_json::from_slice(&fs::read(path)?)?;
        if let Some(protected) = &config.protected_identity {
            anyhow::ensure!(
                Some(protected.kind.as_str()) == crate::identity_store::kind(),
                "This identity requires its original operating system and user account"
            );
            let key = crate::identity_store::get(dir, &protected.reference)?;
            config.private_key = hex::encode(&key);
            anyhow::ensure!(
                hex::encode(Identity::from_config(&config)?.public) == protected.public_key,
                "Protected identity does not match the paired identity"
            );
        }
        config.settings.validate()?;
        Identity::from_config(&config)?;
        return Ok(config);
    }
    let kp = snow::Builder::new(NOISE.parse()?).generate_keypair()?;
    let receive = directories::UserDirs::new()
        .and_then(|d| d.download_dir().map(|p| p.join("Hopping Cursor")))
        .unwrap_or_else(|| dir.join("Received"));
    let config = Config {
        private_key: hex::encode(kp.private),
        protected_identity: None,
        settings: Settings {
            device_name: std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_else(|_| "My computer".into()),
            receive_directory: receive.to_string_lossy().into(),
            ..Settings::default()
        },
        peers: BTreeMap::new(),
        layout: vec![],
        profiles: BTreeMap::new(),
        hop_keys: vec![],
    };
    save(dir, &config)?;
    Ok(config)
}
/// Atomically migrate an existing key, preserving its public identity and peers.
pub fn load_native(dir: &Path) -> Result<Config> {
    let config = load(dir)?;
    protect_with(dir, config, |reference, key| {
        crate::identity_store::put(dir, reference, key)?;
        crate::identity_store::get(dir, reference)
    })
}
fn protect_with(
    dir: &Path,
    mut config: Config,
    store: impl FnOnce(&str, &[u8]) -> Result<Zeroizing<Vec<u8>>>,
) -> Result<Config> {
    if config.protected_identity.is_some() {
        return Ok(config);
    }
    let Some(kind) = crate::identity_store::kind() else {
        return Ok(config);
    };
    let identity = Identity::from_config(&config)?;
    let reference = uuid::Uuid::new_v4().simple().to_string();
    let recovered = store(&reference, &identity.private)?;
    anyhow::ensure!(
        *recovered == *identity.private,
        "Protected identity read-back failed; original config preserved"
    );
    config.protected_identity = Some(ProtectedIdentity {
        kind: kind.into(),
        reference,
        public_key: hex::encode(&identity.public),
    });
    save(dir, &config)?;
    Ok(config)
}
pub fn save(dir: &Path, config: &Config) -> Result<()> {
    config.settings.validate()?;
    let tmp = dir.join(format!("config-{}.tmp", uuid::Uuid::new_v4()));
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    use std::io::Write;
    let mut f = opts.open(&tmp)?;
    let mut disk = config.clone();
    if disk.protected_identity.is_some() {
        use zeroize::Zeroize;
        disk.private_key.zeroize();
        disk.private_key.clear();
    }
    f.write_all(&serde_json::to_vec_pretty(&disk)?)?;
    f.sync_all()?;
    drop(f);
    // Windows rename cannot replace an existing file; use backup rollback there.
    #[cfg(windows)]
    {
        let dst = dir.join("config.json");
        let backup = dir.join("config.previous");
        if dst.exists() {
            if backup.exists() {
                fs::remove_file(&backup)?;
            }
            fs::rename(&dst, &backup)?;
        }
        if let Err(e) = fs::rename(&tmp, &dst) {
            if backup.exists() {
                let _ = fs::rename(&backup, &dst);
            }
            return Err(e.into());
        }
        if backup.exists() {
            fs::remove_file(backup)?;
        }
    }
    #[cfg(not(windows))]
    fs::rename(tmp, dir.join("config.json"))?;
    Ok(())
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod tests {
    use super::*;
    #[test]
    fn protected_migration_preserves_identity_and_removes_plaintext_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let original = load(dir.path()).unwrap();
        let public = Identity::from_config(&original).unwrap().public;
        let protected = protect_with(dir.path(), original.clone(), |_, key| {
            Ok(Zeroizing::new(key.to_vec()))
        })
        .unwrap();
        assert_eq!(Identity::from_config(&protected).unwrap().public, public);
        let disk: Config =
            serde_json::from_slice(&fs::read(dir.path().join("config.json")).unwrap()).unwrap();
        assert!(disk.private_key.is_empty());
        assert!(disk.protected_identity.is_some());
        assert!(!fs::read_to_string(dir.path().join("config.json"))
            .unwrap()
            .contains(&original.private_key));
    }
    #[test]
    fn failed_identity_protection_preserves_original_file() {
        let dir = tempfile::tempdir().unwrap();
        let original = load(dir.path()).unwrap();
        let before = fs::read(dir.path().join("config.json")).unwrap();
        assert!(
            protect_with(dir.path(), original.clone(), |_, _| anyhow::bail!(
                "Store unavailable"
            ))
            .is_err()
        );
        assert_eq!(fs::read(dir.path().join("config.json")).unwrap(), before);
        assert!(
            protect_with(dir.path(), original, |_, _| Ok(Zeroizing::new(vec![0; 32]))).is_err()
        );
        assert_eq!(fs::read(dir.path().join("config.json")).unwrap(), before);
    }
}
