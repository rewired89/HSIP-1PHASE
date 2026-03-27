mod proxy;

use crate::proxy::{run_proxy, Config};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<()> {
    let gateway_config = build_gateway_configuration();

    println!("[gateway] Initializing with configuration: {:?}", gateway_config);

    // Generate proxy auto-config files for easy setup
    if let Err(e) = generate_proxy_config_files(&gateway_config.listen_addr) {
        eprintln!("[gateway] Warning: Failed to generate proxy config files: {}", e);
    }

    run_proxy(gateway_config)
}

fn generate_proxy_config_files(listen_addr: &str) -> Result<()> {
    let config_dir = get_config_directory()?;
    fs::create_dir_all(&config_dir)?;

    // Generate PAC file for automatic proxy configuration
    let pac_content = format!(
        r#"function FindProxyForURL(url, host) {{
    // HSIP Privacy Proxy - Auto-generated configuration
    // Blocks trackers and routes traffic through HSIP gateway

    // Localhost and private networks - direct connection
    if (isPlainHostName(host) ||
        shExpMatch(host, "*.local") ||
        isInNet(dnsResolve(host), "10.0.0.0", "255.0.0.0") ||
        isInNet(dnsResolve(host), "172.16.0.0", "255.240.0.0") ||
        isInNet(dnsResolve(host), "192.168.0.0", "255.255.0.0") ||
        isInNet(dnsResolve(host), "127.0.0.0", "255.0.0.0")) {{
        return "DIRECT";
    }}

    // Route everything else through HSIP gateway
    return "PROXY {}; DIRECT";
}}
"#,
        listen_addr
    );

    let pac_file = config_dir.join("proxy.pac");
    fs::write(&pac_file, pac_content)?;
    println!("[gateway] Generated PAC file: {}", pac_file.display());

    // Generate Windows registry scripts for easy enable/disable
    #[cfg(target_os = "windows")]
    {{
        let enable_script = format!(
            r#"@echo off
echo Enabling HSIP Privacy Proxy...
reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings" /v ProxyEnable /t REG_DWORD /d 1 /f
reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings" /v ProxyServer /t REG_SZ /d "{}" /f
echo HSIP proxy enabled. Please restart your browser.
pause
"#,
            listen_addr
        );

        let disable_script = r#"@echo off
echo Disabling HSIP Privacy Proxy...
reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings" /v ProxyEnable /t REG_DWORD /d 0 /f
reg delete "HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings" /v ProxyServer /f 2>nul
echo HSIP proxy disabled. Please restart your browser.
pause
"#;

        fs::write(config_dir.join("enable-proxy.bat"), enable_script)?;
        fs::write(config_dir.join("disable-proxy.bat"), disable_script)?;
        println!("[gateway] Generated Windows proxy scripts in: {}", config_dir.display());
    }}

    Ok(())
}

fn get_config_directory() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        // Use LocalAppData for Windows
        let local_appdata = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| {
                let userprofile = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
                format!("{}\\AppData\\Local", userprofile)
            });
        Ok(PathBuf::from(local_appdata).join("HSIP").join("gateway"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Use ~/.config/hsip on Unix-like systems
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Ok(PathBuf::from(home).join(".config").join("hsip").join("gateway"))
    }
}

fn build_gateway_configuration() -> Config {
    let listen_address = read_listen_address();
    let connection_timeout = read_timeout_configuration();

    Config {
        listen_addr: listen_address,
        connect_timeout_ms: connection_timeout,
    }
}

fn read_listen_address() -> String {
    std::env::var("HSIP_GATEWAY_LISTEN")
        .unwrap_or_else(|_| String::from("127.0.0.1:8080"))
}

fn read_timeout_configuration() -> u64 {
    std::env::var("HSIP_GATEWAY_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5000)
}
