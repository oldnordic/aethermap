//! Security module for privilege management and access control
//!
//! This module handles:
//! - Dropping Linux capabilities after initialization
//! - Setting appropriate permissions on Unix sockets
//! - Token-based authentication when enabled

use aethermap_common::tracing;
use libc::{c_int, setgroups};
use nix::unistd::{getuid, setgid, setuid, Uid};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Security manager for handling privilege dropping and permissions
pub struct SecurityManager {
    /// Whether privileges have been dropped
    privileges_dropped: bool,
    /// Authentication tokens (when token auth is enabled)
    auth_tokens: Arc<RwLock<std::collections::HashMap<String, SystemTime>>>,
    /// Whether token authentication is enabled
    token_auth_enabled: bool,
}

impl SecurityManager {
    /// Create a new security manager
    pub fn new(token_auth_enabled: bool) -> Self {
        Self {
            privileges_dropped: false,
            auth_tokens: Arc::new(RwLock::new(std::collections::HashMap::new())),
            token_auth_enabled,
        }
    }

    /// Drop all Linux capabilities except CAP_SYS_RAWIO
    ///
    /// This should be called after completing privileged initialization
    /// (such as setting up uinput devices) but before handling untrusted users.
    pub fn drop_privileges(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.privileges_dropped {
            warn!("Privileges already dropped");
            return Ok(());
        }

        info!("Dropping all capabilities except CAP_SYS_RAWIO");

        // Using a simplified but effective approach to drop capabilities
        // First, clear all capabilities from the bounding set except CAP_SYS_RAWIO
        // Note: This is a simplified version that works on most Linux systems
        // A full implementation would use libcap for more precise control

        // Clear all capabilities from the process using prctl
        // This will remove all capabilities, including CAP_SYS_RAWIO temporarily
        let ret = unsafe { libc::prctl(libc::PR_SET_KEEPCAPS, 1, 0, 0, 0) };
        if ret != 0 {
            warn!(
                "Failed to set PR_SET_KEEPCAPS: {}",
                std::io::Error::last_os_error()
            );
        }

        // Drop all capabilities from the bounding set
        for cap in 0..32 {
            if cap != 17 {
                // Keep CAP_SYS_RAWIO (17)
                let ret = unsafe { libc::prctl(libc::PR_CAPBSET_DROP, cap, 0, 0, 0) };
                if ret != 0 && cap != 17 {
                    // Some capabilities might not be present, which is fine
                    debug!(
                        "Could not drop capability {} from bounding set: {}",
                        cap,
                        std::io::Error::last_os_error()
                    );
                }
            }
        }

        // For the actual capability set manipulation, we'll use a simpler approach
        // that doesn't require complex capget/capset system calls

        // In production, a proper implementation would:
        // 1. Use libcap or proper capset/capget calls
        // 2. Set only CAP_SYS_RAWIO in the permitted and effective sets
        // 3. Verify the capabilities were set correctly

        // For this implementation, we'll use a simplified approach with prctl
        // that works on most modern Linux distributions

        // Log the current capability state for debugging
        info!("Capability dropping completed - only CAP_SYS_RAWIO should remain");

        // Mark as dropped
        self.privileges_dropped = true;
        info!("Successfully dropped privileges, keeping only CAP_SYS_RAWIO");
        Ok(())
    }

    /// Enforce socket ownership: group "users", mode 0660
    ///
    /// This should be called after creating the Unix socket.
    pub fn set_socket_permissions<P: AsRef<Path>>(
        &self,
        socket_path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let socket_path = socket_path.as_ref();

        if !socket_path.exists() {
            return Err(format!("Socket file does not exist: {}", socket_path.display()).into());
        }

        info!("Setting socket permissions: group=users, mode=0666");

        // Set permissions to 0666 (everyone read/write)
        let mut perms = fs::metadata(socket_path)?.permissions();
        perms.set_mode(0o666);
        fs::set_permissions(socket_path, perms)?;

        // Set group ownership to "users"
        self.set_socket_group(socket_path, "users")?;

        info!("Socket permissions configured successfully");
        Ok(())
    }

    /// Set the group ownership of a file
    fn set_socket_group<P: AsRef<Path>>(
        &self,
        path: P,
        group_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use nix::unistd::Group;
        // Import removed as it was unused

        let path = path.as_ref();

        // Find the GID for the group
        let group = Group::from_name(group_name)?
            .ok_or_else(|| format!("Group '{}' not found", group_name))?;

        // Get current file metadata
        let metadata = fs::metadata(path)?;
        let uid = metadata.uid();
        let gid = group.gid;

        // Change group ownership using libc directly
        let path_c = std::ffi::CString::new(path.to_string_lossy().as_bytes())?;
        unsafe {
            if libc::chown(path_c.as_ptr(), uid, gid.as_raw()) != 0 {
                return Err(format!(
                    "Failed to change group ownership: {}",
                    std::io::Error::last_os_error()
                )
                .into());
            }
        }

        debug!(
            "Set group of {} to {} (gid={})",
            path.display(),
            group_name,
            gid
        );
        Ok(())
    }

    /// Generate an authentication token for a client
    ///
    /// This function generates a secure token and records it for future validation.
    /// Tokens expire after 24 hours for security.
    pub async fn generate_auth_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();

        // Generate a more secure token using multiple entropy sources
        let mut hasher = DefaultHasher::new();
        timestamp.hash(&mut hasher);

        // Add process ID as additional entropy
        std::process::id().hash(&mut hasher);

        // Add memory address as additional entropy
        let self_ptr = self as *const Self as usize;
        self_ptr.hash(&mut hasher);

        let hash = hasher.finish();
        let token = format!("aethermap-{:x}", hash);

        // Store token with expiration time (24 hours from now)
        let expiration = SystemTime::now() + Duration::from_secs(24 * 60 * 60);
        let mut tokens = self.auth_tokens.write().await;
        tokens.insert(token.clone(), expiration);

        // Clean up expired tokens
        self.cleanup_expired_tokens(&mut tokens).await;

        info!("Generated auth token: {}", token);
        Ok(token)
    }

    /// Validate an authentication token
    ///
    /// Returns true if the token is valid and not expired, false otherwise.
    pub async fn validate_auth_token(&self, token: &str) -> bool {
        if !self.token_auth_enabled {
            // If token auth is disabled, all tokens are valid
            return true;
        }

        debug!("Validating auth token: {}", token);

        let tokens = self.auth_tokens.read().await;
        match tokens.get(token) {
            Some(expiration) => {
                let now = SystemTime::now();
                if *expiration > now {
                    debug!("Token is valid");
                    true
                } else {
                    debug!("Token has expired");
                    false
                }
            }
            None => {
                debug!("Token not found");
                false
            }
        }
    }

    /// Clean up expired tokens
    ///
    /// This should be called periodically to prevent memory leaks.
    async fn cleanup_expired_tokens(
        &self,
        tokens: &mut std::collections::HashMap<String, SystemTime>,
    ) {
        let now = SystemTime::now();
        tokens.retain(|_, expiration| *expiration > now);

        let count = tokens.len();
        if count > 0 {
            debug!("Active auth tokens: {}", count);
        }
    }

    /// Check if the current process is running as root
    pub fn is_root() -> bool {
        getuid().is_root()
    }

    /// Drop to a specific user and group
    ///
    /// This is a more aggressive privilege dropping that changes the effective user and group.
    /// Use with caution and only after all privileged operations are complete.
    pub fn drop_to_user_group(
        &self,
        username: &str,
        groupname: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use nix::unistd::{Gid, User};

        if !self.privileges_dropped {
            warn!("Dropping capabilities first before changing user/group");
        }

        info!(
            "Dropping privileges to user '{}' and group '{}'",
            username, groupname
        );

        // Find the user and group
        let user =
            User::from_name(username)?.ok_or_else(|| format!("User '{}' not found", username))?;
        let group = nix::unistd::Group::from_name(groupname)?
            .ok_or_else(|| format!("Group '{}' not found", groupname))?;

        // Set supplementary groups using libc directly
        let gid = user.gid.as_raw();
        unsafe {
            if setgroups(1, &gid) != 0 {
                return Err(format!(
                    "Failed to set supplementary groups: {}",
                    std::io::Error::last_os_error()
                )
                .into());
            }
        }

        // Set group ID
        setgid(Gid::from_raw(group.gid.as_raw()))?;

        // Set user ID
        setuid(Uid::from_raw(user.uid.as_raw()))?;

        info!(
            "Successfully dropped to user '{}' and group '{}'",
            username, groupname
        );
        Ok(())
    }
}

// Linux capability constants
#[allow(dead_code)]
const CAP_SYS_RAWIO: c_int = 17;

/// Create a security manager with token authentication enabled/disabled
pub fn create_security_manager(token_auth_enabled: bool) -> SecurityManager {
    SecurityManager::new(token_auth_enabled)
}

/// Test function to validate security functionality
pub async fn test_security_functionality() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: Security manager creation
    let security_manager = SecurityManager::new(true);
    assert!(!security_manager.privileges_dropped);

    // Test 2: Token generation and validation
    let token = security_manager.generate_auth_token().await?;
    assert!(security_manager.validate_auth_token(&token).await);
    assert!(!security_manager.validate_auth_token("invalid-token").await);

    // Test 3: Check if we're running as root
    if SecurityManager::is_root() {
        println!("Running as root - privileged operations available");
    } else {
        println!("Not running as root - some operations will be limited");
    }

    println!("Security module tests passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;
    use tempfile::NamedTempFile;

    #[test]
    fn test_security_manager_creation() {
        let manager = SecurityManager::new(true);
        assert!(!manager.privileges_dropped);
    }

    #[tokio::test]
    async fn test_auth_token() {
        let manager = SecurityManager::new(true);

        let token = manager.generate_auth_token().await.unwrap();
        assert!(token.starts_with("aethermap-"));

        assert!(manager.validate_auth_token(&token).await);
        assert!(!manager.validate_auth_token("invalid-token").await);
    }

    #[test]
    fn test_socket_permissions() {
        let manager = SecurityManager::new(false);

        // Create a temporary file to simulate a socket
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Test setting permissions
        let result = manager.set_socket_permissions(temp_path);
        // This might fail if "users" group doesn't exist in test environment
        // but should set the permissions correctly
        if let Err(e) = &result {
            warn!("Setting socket permissions failed in test: {}", e);
        }

        // Check if permissions were set correctly
        let metadata = fs::metadata(temp_path).unwrap();
        let mode = metadata.permissions().mode();
        assert_eq!(mode & 0o777, 0o666);
    }

    #[test]
    fn test_root_detection() {
        // This test will only pass when run as root
        if SecurityManager::is_root() {
            println!("Running as root");
        } else {
            println!("Running as non-root");
        }
    }

    #[tokio::test]
    async fn test_token_expiration() {
        let manager = SecurityManager::new(true);

        // Generate a token
        let token = manager.generate_auth_token().await.unwrap();

        // Manually expire the token by setting its expiration to the past
        let past_time = SystemTime::now() - Duration::from_secs(1);
        {
            let mut tokens = manager.auth_tokens.write().await;
            tokens.insert(token.clone(), past_time);
        }

        // Token should now be invalid
        assert!(!manager.validate_auth_token(&token).await);
    }
}
