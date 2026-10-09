//! OS-protected identity storage. Losing access is an error, never a key rotation.
#[cfg(any(target_os = "macos", target_os = "windows"))]
use anyhow::Context;
use anyhow::Result;
use std::path::Path;
use zeroize::Zeroizing;

#[cfg(target_os = "macos")]
const SERVICE: &str = "com.Itskorrah.TheHoppingCursor.identity";

pub(crate) fn kind() -> Option<&'static str> {
    #[cfg(target_os = "macos")]
    {
        Some("macos-keychain")
    }
    #[cfg(target_os = "windows")]
    {
        Some("windows-dpapi")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub(crate) fn validate_reference(reference: &str) -> Result<()> {
    anyhow::ensure!(
        reference.len() == 32 && reference.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid protected identity reference"
    );
    Ok(())
}
#[cfg(target_os = "macos")]
pub(crate) fn put(_dir: &Path, reference: &str, key: &[u8]) -> Result<()> {
    validate_reference(reference)?;
    security_framework::passwords::set_generic_password(SERVICE, reference, key)
        .context("Could not save identity in macOS Keychain")
}
#[cfg(target_os = "macos")]
pub(crate) fn get(_dir: &Path, reference: &str) -> Result<Zeroizing<Vec<u8>>> {
    validate_reference(reference)?;
    Ok(Zeroizing::new(
        security_framework::passwords::get_generic_password(SERVICE, reference)
            .context("Cannot unlock identity in macOS Keychain; no new identity was generated")?,
    ))
}
#[cfg(target_os = "windows")]
fn crypt(data: &[u8], protect: bool) -> Result<Zeroizing<Vec<u8>>> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };
    let input = CRYPT_INTEGER_BLOB {
        cbData: data.len().try_into()?,
        pbData: data.as_ptr() as *mut u8,
    };
    let entropy = b"TheHoppingCursor/identity/v1";
    let entropy = CRYPT_INTEGER_BLOB {
        cbData: entropy.len() as u32,
        pbData: entropy.as_ptr() as *mut u8,
    };
    let mut output: CRYPT_INTEGER_BLOB = unsafe { std::mem::zeroed() };
    let ok = unsafe {
        if protect {
            CryptProtectData(
                &input,
                std::ptr::null(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                &entropy,
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
    };
    anyhow::ensure!(
        ok != 0,
        "Windows DPAPI could not protect/unlock the identity: {}",
        std::io::Error::last_os_error()
    );
    unsafe {
        let bytes = std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize);
        let result = Zeroizing::new(bytes.to_vec());
        use zeroize::Zeroize;
        bytes.zeroize();
        LocalFree(output.pbData as _);
        Ok(result)
    }
}
#[cfg(target_os = "windows")]
pub(crate) fn put(dir: &Path, reference: &str, key: &[u8]) -> Result<()> {
    use std::io::Write;
    validate_reference(reference)?;
    let encrypted = crypt(key, true)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(format!("identity-{reference}.dpapi")))?;
    file.write_all(&encrypted)?;
    file.sync_all()?;
    Ok(())
}
#[cfg(target_os = "windows")]
pub(crate) fn get(dir: &Path, reference: &str) -> Result<Zeroizing<Vec<u8>>> {
    validate_reference(reference)?;
    let path = dir.join(format!("identity-{reference}.dpapi"));
    let length = std::fs::metadata(&path)?.len();
    anyhow::ensure!(length <= 4096, "Invalid protected identity size");
    crypt(
        &std::fs::read(path).context("Missing DPAPI identity; no replacement was generated")?,
        false,
    )
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn put(_: &Path, _: &str, _: &[u8]) -> Result<()> {
    anyhow::bail!("OS identity storage is unavailable on this platform")
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(crate) fn get(_: &Path, _: &str) -> Result<Zeroizing<Vec<u8>>> {
    anyhow::bail!("OS identity storage is unavailable on this platform")
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::*;
    #[test]
    fn windows_dpapi_round_trip_tamper_and_non_overwrite() {
        let directory = tempfile::tempdir().unwrap();
        let reference = uuid::Uuid::new_v4().simple().to_string();
        let synthetic_key = [0xa5; 32];
        put(directory.path(), &reference, &synthetic_key).unwrap();
        assert_eq!(
            &**get(directory.path(), &reference).unwrap(),
            &synthetic_key
        );
        assert!(put(directory.path(), &reference, &[0; 32]).is_err());
        assert!(get(directory.path(), "../invalid").is_err());
        let path = directory.path().join(format!("identity-{reference}.dpapi"));
        let mut encrypted = std::fs::read(&path).unwrap();
        assert!(!encrypted.windows(32).any(|part| part == synthetic_key));
        let last = encrypted.len() - 1;
        encrypted[last] ^= 1;
        std::fs::write(path, encrypted).unwrap();
        assert!(get(directory.path(), &reference).is_err());
    }
}
