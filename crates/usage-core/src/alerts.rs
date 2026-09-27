//! Budget alert levels the user can set (Settings → Alerts): at what share of the budget used the
//! tray ring, the panel's budget bar and the ccline status line turn to the warning color, and then
//! to the critical color. Stored in the settings file, so every app agrees.

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Alerts {
    /// Percent of the budget used where the warning color starts.
    pub warning_pct: u8,
    /// `#rrggbb`
    pub warning_color: String,
    /// Percent of the budget used where the critical color starts (above `warning_pct`).
    pub critical_pct: u8,
    pub critical_color: String,
}

impl Default for Alerts {
    fn default() -> Alerts {
        Alerts {
            warning_pct: 75,
            warning_color: "#ffd60a".into(),
            critical_pct: 90,
            critical_color: "#ff453a".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Normal,
    Warning,
    Critical,
}

/// Below the warning level (theme blue).
pub const NORMAL_RGB: [u8; 3] = [10, 132, 255];

impl Alerts {
    /// Which level a share of the budget used (0.0–1.0+) falls in.
    pub fn level(&self, used_fraction: f64) -> Level {
        let pct = used_fraction * 100.0;
        if pct >= self.critical_pct as f64 {
            Level::Critical
        } else if pct >= self.warning_pct as f64 {
            Level::Warning
        } else {
            Level::Normal
        }
    }

    /// The color for a share of the budget used.
    pub fn rgb(&self, used_fraction: f64) -> [u8; 3] {
        match self.level(used_fraction) {
            Level::Normal => NORMAL_RGB,
            Level::Warning => parse_hex(&self.warning_color).unwrap_or([255, 214, 10]),
            Level::Critical => parse_hex(&self.critical_color).unwrap_or([255, 69, 58]),
        }
    }

    /// Checks values coming from the Settings view.
    pub fn validate(&self) -> Result<()> {
        if !(1..=100).contains(&self.warning_pct) || !(1..=100).contains(&self.critical_pct) {
            return Err(Error::Invalid("Alert levels must be between 1% and 100%.".into()));
        }
        if self.warning_pct >= self.critical_pct {
            return Err(Error::Invalid("The warning level must be below the critical level.".into()));
        }
        for c in [&self.warning_color, &self.critical_color] {
            if parse_hex(c).is_none() {
                return Err(Error::Invalid(format!("Not a color: {c}")));
            }
        }
        Ok(())
    }
}

/// `#rrggbb` → RGB.
pub fn parse_hex(color: &str) -> Option<[u8; 3]> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_the_thresholds() {
        let a = Alerts { warning_pct: 50, critical_pct: 80, ..Alerts::default() };
        assert_eq!(a.level(0.49), Level::Normal);
        assert_eq!(a.level(0.5), Level::Warning);
        assert_eq!(a.level(0.8), Level::Critical);
        assert_eq!(a.level(1.3), Level::Critical);
        assert_eq!(a.rgb(0.1), NORMAL_RGB);
        assert_eq!(a.rgb(0.6), [0xff, 0xd6, 0x0a]);
    }

    #[test]
    fn validates_settings_input() {
        assert!(Alerts::default().validate().is_ok());
        assert!(Alerts { warning_pct: 90, critical_pct: 90, ..Alerts::default() }.validate().is_err());
        assert!(Alerts { critical_pct: 0, ..Alerts::default() }.validate().is_err());
        assert!(Alerts { warning_color: "yellow".into(), ..Alerts::default() }.validate().is_err());
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let a: Alerts = serde_json::from_str(r#"{"warning_pct": 60}"#).unwrap();
        assert_eq!(a.warning_pct, 60);
        assert_eq!(a.critical_pct, 90);
    }
}
