/// Windows registry settings for rzr.
/// All settings stored in HKCU\SOFTWARE\rzr

use winreg::enums::*;
use winreg::RegKey;

const REG_PATH: &str = "SOFTWARE\\rzr";

pub struct Settings {
    pub eq_enabled: bool,
    pub eq_bands: [i8; 10],
    pub volume: u8,
    pub enhancement: bool,
    pub wait_timeout_ms: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            eq_enabled: true,
            eq_bands: [1, -2, 1, -3, 1, -3, -5, 2, 2, 3],
            volume: 255,
            enhancement: true,
            wait_timeout_ms: 5000,
        }
    }
}

impl Settings {
    /// Load settings from registry. Missing keys get defaults.
    pub fn load() -> Self {
        let mut s = Self::default();
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = match hkcu.open_subkey(REG_PATH) {
            Ok(k) => k,
            Err(_) => return s,
        };

        if let Ok(v) = key.get_value::<u32, _>("eq_enabled") {
            s.eq_enabled = v != 0;
        }
        if let Ok(v) = key.get_value::<String, _>("eq_bands") {
            if let Some(bands) = parse_eq_bands(&v) {
                s.eq_bands = bands;
            }
        }
        if let Ok(v) = key.get_value::<u32, _>("volume") {
            s.volume = v.min(255) as u8;
        }
        if let Ok(v) = key.get_value::<u32, _>("enhancement") {
            s.enhancement = v != 0;
        }
        if let Ok(v) = key.get_value::<u32, _>("wait_timeout_ms") {
            s.wait_timeout_ms = v;
        }

        s
    }

    /// Save all settings to registry.
    pub fn save(&self) -> Result<(), String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu
            .create_subkey(REG_PATH)
            .map_err(|e| format!("Cannot create registry key: {e}"))?;

        key.set_value("eq_enabled", &(self.eq_enabled as u32))
            .map_err(|e| format!("Registry write failed: {e}"))?;
        key.set_value("eq_bands", &format_eq_bands(&self.eq_bands))
            .map_err(|e| format!("Registry write failed: {e}"))?;
        key.set_value("volume", &(self.volume as u32))
            .map_err(|e| format!("Registry write failed: {e}"))?;
        key.set_value("enhancement", &(self.enhancement as u32))
            .map_err(|e| format!("Registry write failed: {e}"))?;
        key.set_value("wait_timeout_ms", &self.wait_timeout_ms)
            .map_err(|e| format!("Registry write failed: {e}"))?;

        Ok(())
    }
}

/// Parse "1,-2,1,-3,1,-3,-5,2,2,3" into [i8; 10].
pub fn parse_eq_bands(s: &str) -> Option<[i8; 10]> {
    let parts: Vec<&str> = s.split(',').map(|p| p.trim()).collect();
    if parts.len() != 10 {
        return None;
    }
    let mut bands = [0i8; 10];
    for (i, part) in parts.iter().enumerate() {
        bands[i] = part.parse::<i8>().ok()?;
    }
    Some(bands)
}

pub fn format_eq_bands(bands: &[i8; 10]) -> String {
    bands
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// EQ presets.
pub const PRESET_FLAT: [i8; 10] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
pub const PRESET_GAME: [i8; 10] = [-3, -3, -4, 0, 5, 5, 4, 1, 0, -1];
pub const PRESET_MUSIC: [i8; 10] = [2, 2, 1, 1, 2, 3, 3, 3, 1, 0];
pub const PRESET_MOVIE: [i8; 10] = [4, 4, 3, 0, -3, -1, 3, 5, 2, 1];
pub const _PRESET_CUSTOM: [i8; 10] = [1, -2, 1, -3, 1, -3, -5, 2, 2, 3];
