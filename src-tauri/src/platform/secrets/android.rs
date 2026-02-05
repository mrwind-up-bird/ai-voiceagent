//! Android Keystore implementation
//!
//! Uses the Android Keystore system to securely store API keys with:
//! - Hardware-backed encryption on supported devices
//! - TEE (Trusted Execution Environment) protection
//! - StrongBox support when available
//!
//! This implementation uses JNI to call Android's KeyStore and EncryptedSharedPreferences APIs.

use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::jboolean;
use jni::JNIEnv;
use std::sync::OnceLock;

use super::{SecureStorage, SecureStorageError, SERVICE_NAME};

/// Global JVM reference for Android
static JAVA_VM: OnceLock<jni::JavaVM> = OnceLock::new();

/// Initialize the JVM reference (called from Tauri's Android setup)
pub fn init_jvm(vm: jni::JavaVM) {
    let _ = JAVA_VM.set(vm);
}

/// Android Keystore secure storage implementation
pub struct AndroidKeystore {
    preference_name: String,
}

impl AndroidKeystore {
    /// Create a new Android Keystore storage instance
    pub fn new() -> Self {
        Self {
            preference_name: SERVICE_NAME.replace('.', "_"),
        }
    }

    /// Create with a custom preference name (for testing)
    #[allow(dead_code)]
    pub fn with_service(service: &str) -> Self {
        Self {
            preference_name: service.replace('.', "_"),
        }
    }

    /// Get JNI environment
    fn get_env(&self) -> Result<jni::AttachGuard<'static>, SecureStorageError> {
        let vm = JAVA_VM
            .get()
            .ok_or_else(|| SecureStorageError::Platform("JVM not initialized".to_string()))?;

        vm.attach_current_thread()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to attach to JVM: {}", e)))
    }

    /// Get or create EncryptedSharedPreferences instance
    fn get_encrypted_prefs<'a>(&self, env: &mut JNIEnv<'a>) -> Result<JObject<'a>, SecureStorageError> {
        // Get the application context
        let activity_thread = env
            .call_static_method(
                "android/app/ActivityThread",
                "currentActivityThread",
                "()Landroid/app/ActivityThread;",
                &[],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get ActivityThread: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert ActivityThread: {}", e)))?;

        let context = env
            .call_method(
                activity_thread,
                "getApplication",
                "()Landroid/app/Application;",
                &[],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get Application: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert Application: {}", e)))?;

        // Create master key
        let master_key_alias = env
            .new_string("_aurus_master_key_")
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create string: {}", e)))?;

        let key_scheme = env
            .get_static_field(
                "androidx/security/crypto/MasterKey$KeyScheme",
                "AES256_GCM",
                "Landroidx/security/crypto/MasterKey$KeyScheme;",
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get KeyScheme: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert KeyScheme: {}", e)))?;

        let master_key_builder = env
            .new_object(
                "androidx/security/crypto/MasterKey$Builder",
                "(Landroid/content/Context;)V",
                &[JValue::Object(&context)],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create MasterKey.Builder: {}", e)))?;

        let master_key_builder = env
            .call_method(
                master_key_builder,
                "setKeyScheme",
                "(Landroidx/security/crypto/MasterKey$KeyScheme;)Landroidx/security/crypto/MasterKey$Builder;",
                &[JValue::Object(&key_scheme)],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to set KeyScheme: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert Builder: {}", e)))?;

        let master_key = env
            .call_method(
                master_key_builder,
                "build",
                "()Landroidx/security/crypto/MasterKey;",
                &[],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to build MasterKey: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert MasterKey: {}", e)))?;

        // Create EncryptedSharedPreferences
        let pref_name = env
            .new_string(&self.preference_name)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create pref name: {}", e)))?;

        let pref_key_scheme = env
            .get_static_field(
                "androidx/security/crypto/EncryptedSharedPreferences$PrefKeyEncryptionScheme",
                "AES256_SIV",
                "Landroidx/security/crypto/EncryptedSharedPreferences$PrefKeyEncryptionScheme;",
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get PrefKeyEncryptionScheme: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert PrefKeyEncryptionScheme: {}", e)))?;

        let pref_value_scheme = env
            .get_static_field(
                "androidx/security/crypto/EncryptedSharedPreferences$PrefValueEncryptionScheme",
                "AES256_GCM",
                "Landroidx/security/crypto/EncryptedSharedPreferences$PrefValueEncryptionScheme;",
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get PrefValueEncryptionScheme: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert PrefValueEncryptionScheme: {}", e)))?;

        let encrypted_prefs = env
            .call_static_method(
                "androidx/security/crypto/EncryptedSharedPreferences",
                "create",
                "(Landroid/content/Context;Ljava/lang/String;Landroidx/security/crypto/MasterKey;Landroidx/security/crypto/EncryptedSharedPreferences$PrefKeyEncryptionScheme;Landroidx/security/crypto/EncryptedSharedPreferences$PrefValueEncryptionScheme;)Landroid/content/SharedPreferences;",
                &[
                    JValue::Object(&context),
                    JValue::Object(&pref_name),
                    JValue::Object(&master_key),
                    JValue::Object(&pref_key_scheme),
                    JValue::Object(&pref_value_scheme),
                ],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create EncryptedSharedPreferences: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert SharedPreferences: {}", e)))?;

        Ok(encrypted_prefs)
    }
}

impl Default for AndroidKeystore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for AndroidKeystore {
    fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError> {
        let mut env = self.get_env()?;
        let prefs = self.get_encrypted_prefs(&mut env)?;

        let editor = env
            .call_method(prefs, "edit", "()Landroid/content/SharedPreferences$Editor;", &[])
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get editor: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert editor: {}", e)))?;

        let key_str = env
            .new_string(key)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create key string: {}", e)))?;
        let value_str = env
            .new_string(value)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create value string: {}", e)))?;

        let editor = env
            .call_method(
                editor,
                "putString",
                "(Ljava/lang/String;Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;",
                &[JValue::Object(&key_str), JValue::Object(&value_str)],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to put string: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert editor: {}", e)))?;

        env.call_method(editor, "apply", "()V", &[])
            .map_err(|e| SecureStorageError::Platform(format!("Failed to apply: {}", e)))?;

        tracing::info!("Stored key '{}' in Android EncryptedSharedPreferences", key);
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError> {
        let mut env = self.get_env()?;
        let prefs = self.get_encrypted_prefs(&mut env)?;

        let key_str = env
            .new_string(key)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create key string: {}", e)))?;

        let result = env
            .call_method(
                prefs,
                "getString",
                "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
                &[JValue::Object(&key_str), JValue::Object(&JObject::null())],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get string: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert result: {}", e)))?;

        if result.is_null() {
            Ok(None)
        } else {
            let jstring = JString::from(result);
            let value: String = env
                .get_string(&jstring)
                .map_err(|e| SecureStorageError::Platform(format!("Failed to convert string: {}", e)))?
                .into();
            tracing::debug!("Retrieved key '{}' from EncryptedSharedPreferences", key);
            Ok(Some(value))
        }
    }

    fn delete(&self, key: &str) -> Result<(), SecureStorageError> {
        let mut env = self.get_env()?;
        let prefs = self.get_encrypted_prefs(&mut env)?;

        let editor = env
            .call_method(prefs, "edit", "()Landroid/content/SharedPreferences$Editor;", &[])
            .map_err(|e| SecureStorageError::Platform(format!("Failed to get editor: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert editor: {}", e)))?;

        let key_str = env
            .new_string(key)
            .map_err(|e| SecureStorageError::Platform(format!("Failed to create key string: {}", e)))?;

        let editor = env
            .call_method(
                editor,
                "remove",
                "(Ljava/lang/String;)Landroid/content/SharedPreferences$Editor;",
                &[JValue::Object(&key_str)],
            )
            .map_err(|e| SecureStorageError::Platform(format!("Failed to remove: {}", e)))?
            .l()
            .map_err(|e| SecureStorageError::Platform(format!("Failed to convert editor: {}", e)))?;

        env.call_method(editor, "apply", "()V", &[])
            .map_err(|e| SecureStorageError::Platform(format!("Failed to apply: {}", e)))?;

        tracing::info!("Deleted key '{}' from EncryptedSharedPreferences", key);
        Ok(())
    }
}
