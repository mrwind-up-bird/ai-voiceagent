//! Windows Credential Manager implementation
//!
//! Uses the Windows Credential Manager (DPAPI) to securely store API keys with:
//! - DPAPI encryption tied to the user account
//! - Automatic credential roaming (if enabled)
//! - Per-application credential isolation

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows::core::PCWSTR;
use windows::Win32::Foundation::ERROR_NOT_FOUND;
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

use super::{SecureStorage, SecureStorageError, SERVICE_NAME};

/// Windows Credential Manager secure storage implementation
pub struct WindowsCredentialManager {
    service: String,
}

impl WindowsCredentialManager {
    /// Create a new Credential Manager storage instance
    pub fn new() -> Self {
        Self {
            service: SERVICE_NAME.to_string(),
        }
    }

    /// Create with a custom service name (for testing)
    #[allow(dead_code)]
    pub fn with_service(service: &str) -> Self {
        Self {
            service: service.to_string(),
        }
    }

    /// Build the target name for a credential
    fn target_name(&self, key: &str) -> Vec<u16> {
        let target = format!("{}:{}", self.service, key);
        OsStr::new(&target)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

impl Default for WindowsCredentialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for WindowsCredentialManager {
    fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError> {
        let target = self.target_name(key);
        let value_bytes = value.as_bytes();

        let credential = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: PCWSTR(target.as_ptr()),
            Comment: PCWSTR::null(),
            LastWritten: Default::default(),
            CredentialBlobSize: value_bytes.len() as u32,
            CredentialBlob: value_bytes.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: PCWSTR::null(),
            UserName: PCWSTR::null(),
        };

        unsafe {
            CredWriteW(&credential, 0).map_err(|e| {
                SecureStorageError::Platform(format!("CredWriteW failed: {}", e))
            })?;
        }

        tracing::info!("Stored key '{}' in Windows Credential Manager", key);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError> {
        let target = self.target_name(key);
        let mut credential_ptr: *mut CREDENTIALW = std::ptr::null_mut();

        unsafe {
            match CredReadW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0, &mut credential_ptr) {
                Ok(()) => {
                    let credential = &*credential_ptr;
                    let value = if credential.CredentialBlobSize > 0 && !credential.CredentialBlob.is_null() {
                        let slice = std::slice::from_raw_parts(
                            credential.CredentialBlob,
                            credential.CredentialBlobSize as usize,
                        );
                        Some(String::from_utf8_lossy(slice).to_string())
                    } else {
                        None
                    };

                    CredFree(credential_ptr as *mut _);
                    tracing::debug!("Retrieved key '{}' from Credential Manager", key);
                    Ok(value)
                }
                Err(e) => {
                    if e.code() == ERROR_NOT_FOUND.into() {
                        Ok(None)
                    } else {
                        Err(SecureStorageError::Platform(format!("CredReadW failed: {}", e)))
                    }
                }
            }
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecureStorageError> {
        let target = self.target_name(key);

        unsafe {
            match CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, 0) {
                Ok(()) => {
                    tracing::info!("Deleted key '{}' from Credential Manager", key);
                    Ok(())
                }
                Err(e) => {
                    if e.code() == ERROR_NOT_FOUND.into() {
                        Ok(()) // Already deleted
                    } else {
                        Err(SecureStorageError::Platform(format!("CredDeleteW failed: {}", e)))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_manager_roundtrip() {
        let storage = WindowsCredentialManager::with_service("com.aurusvoiceintelligence.test");
        let key = "test_api_key";
        let value = "sk-test-12345";

        // Clean up any existing test data
        let _ = storage.delete(key);

        // Set and get
        storage.set(key, value).expect("Failed to set");
        let retrieved = storage.get(key).expect("Failed to get");
        assert_eq!(retrieved, Some(value.to_string()));

        // Delete
        storage.delete(key).expect("Failed to delete");
        let after_delete = storage.get(key).expect("Failed to get after delete");
        assert_eq!(after_delete, None);
    }
}
