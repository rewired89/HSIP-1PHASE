//! Geolocation lookup for IP addresses
//!
//! Uses MaxMind GeoLite2 database for IP to location mapping.

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;

#[cfg(feature = "geolocation")]
use maxminddb::geoip2;

/// Geographic location information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Country name
    pub country: Option<String>,
    /// Country ISO code
    pub country_code: Option<String>,
    /// City name
    pub city: Option<String>,
    /// Latitude
    pub latitude: Option<f64>,
    /// Longitude
    pub longitude: Option<f64>,
    /// Time zone
    pub timezone: Option<String>,
    /// Continent
    pub continent: Option<String>,
}

impl Default for GeoLocation {
    fn default() -> Self {
        Self {
            country: None,
            country_code: None,
            city: None,
            latitude: None,
            longitude: None,
            timezone: None,
            continent: None,
        }
    }
}

/// Geolocation database wrapper
#[cfg(feature = "geolocation")]
pub struct GeoLocator {
    reader: Arc<maxminddb::Reader<Vec<u8>>>,
}

#[cfg(feature = "geolocation")]
impl GeoLocator {
    /// Create a new GeoLocator with database path
    pub fn new(db_path: PathBuf) -> Result<Self, String> {
        let reader = maxminddb::Reader::open_readfile(db_path)
            .map_err(|e| format!("Failed to open GeoIP database: {}", e))?;

        Ok(Self {
            reader: Arc::new(reader),
        })
    }

    /// Look up geolocation for an IP address
    pub fn lookup(&self, ip: IpAddr) -> Result<GeoLocation, String> {
        let result = self
            .reader
            .lookup(ip)
            .map_err(|e| format!("GeoIP lookup failed: {}", e))?;

        let city: geoip2::City = result
            .decode()
            .map_err(|e| format!("GeoIP decode failed: {}", e))?
            .ok_or_else(|| format!("No GeoIP data found for {}", ip))?;

        let country = city.country.names.english
            .map(|s: &str| s.to_string());

        let country_code = city.country.iso_code
            .map(|s: &str| s.to_string());

        let city_name = city.city.names.english
            .map(|s: &str| s.to_string());

        let timezone = city.location.time_zone
            .map(|s: &str| s.to_string());

        let continent = city.continent.names.english
            .map(|s: &str| s.to_string());

        Ok(GeoLocation {
            country,
            country_code,
            city: city_name,
            latitude: city.location.latitude,
            longitude: city.location.longitude,
            timezone,
            continent,
        })
    }

    /// Batch lookup for multiple IPs
    pub fn lookup_batch(&self, ips: &[IpAddr]) -> Vec<(IpAddr, Option<GeoLocation>)> {
        ips.iter()
            .map(|ip| (*ip, self.lookup(*ip).ok()))
            .collect()
    }
}

#[cfg(not(feature = "geolocation"))]
pub struct GeoLocator;

#[cfg(not(feature = "geolocation"))]
impl GeoLocator {
    pub fn new(_db_path: PathBuf) -> Result<Self, String> {
        Err("Geolocation support not enabled. Compile with --features geolocation".to_string())
    }

    pub fn lookup(&self, _ip: IpAddr) -> Result<GeoLocation, String> {
        Err("Geolocation not available".to_string())
    }
}

/// Helper to download GeoLite2 database (requires free MaxMind account)
pub mod download {
    use std::path::Path;

    /// Instructions for obtaining GeoLite2 database
    pub fn instructions() -> &'static str {
        r#"
To enable geolocation:

1. Sign up for a free MaxMind account: https://www.maxmind.com/en/geolite2/signup
2. Download GeoLite2-City.mmdb
3. Place it in one of these locations:
   - /var/lib/hsip/GeoLite2-City.mmdb (Linux)
   - C:\ProgramData\HSIP\GeoLite2-City.mmdb (Windows)
   - ~/Library/Application Support/HSIP/GeoLite2-City.mmdb (macOS)
4. Restart HSIP with --features geolocation

Alternatively, set environment variable:
   HSIP_GEOIP_DB=/path/to/GeoLite2-City.mmdb
"#
    }

    /// Default database path for platform
    pub fn default_path() -> String {
        if cfg!(target_os = "windows") {
            "C:\\ProgramData\\HSIP\\GeoLite2-City.mmdb".to_string()
        } else if cfg!(target_os = "macos") {
            "~/Library/Application Support/HSIP/GeoLite2-City.mmdb".to_string()
        } else {
            "/var/lib/hsip/GeoLite2-City.mmdb".to_string()
        }
    }

    /// Check if database exists at default path
    pub fn database_exists() -> bool {
        Path::new(&default_path()).exists()
            || std::env::var("HSIP_GEOIP_DB")
                .ok()
                .map(|p| Path::new(&p).exists())
                .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geolocation_default() {
        let geo = GeoLocation::default();
        assert!(geo.country.is_none());
        assert!(geo.latitude.is_none());
    }

    #[test]
    fn test_download_instructions() {
        let instructions = download::instructions();
        assert!(instructions.contains("MaxMind"));
        assert!(instructions.contains("GeoLite2"));
    }
}
