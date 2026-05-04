use anyhow::{bail, Context, Result};

/// Return the path where hsip-api writes the admin key on this platform.
/// Mirrors hsip_data_dir() in hsip-api/src/config.rs.
pub fn admin_key_path() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return std::path::PathBuf::from(appdata)
                .join("HSIP")
                .join("admin.key");
        }
    }
    // Linux / macOS / fallback
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".hsip")
        .join("admin.key")
}

pub fn load_admin_key() -> Result<String> {
    let path = admin_key_path();
    if !path.exists() {
        bail!(
            "No API key found.\n\
             Provide --key or set the HSIP_API_KEY environment variable.\n\
             Or start HSIP first — it writes the key to: {}",
            path.display()
        );
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let key = raw.trim().to_string();
    if key.is_empty() {
        bail!(
            "Admin key file is empty: {}\nStart HSIP to regenerate it.",
            path.display()
        );
    }
    Ok(key)
}
