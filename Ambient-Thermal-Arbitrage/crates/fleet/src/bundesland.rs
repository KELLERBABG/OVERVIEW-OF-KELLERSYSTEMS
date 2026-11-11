// ============================================================================
// 16 German Federal States (Bundesländer) — Geo, Climate & Grid Profiles
// ============================================================================
// Supports full regional dispatch, weather retrieval (Open-Meteo),
// solar irradiance geometry, and VNB concession windows (§ 19 StromNEV).
// ============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Bundesland {
    BadenWuerttemberg,
    Bayern,
    Berlin,
    Brandenburg,
    Bremen,
    Hamburg,
    Hessen,
    MecklenburgVorpommern,
    Niedersachsen,
    NordrheinWestfalen,
    RheinlandPfalz,
    Saarland,
    Sachsen,
    SachsenAnhalt,
    SchleswigHolstein,
    Thueringen,
}

impl Bundesland {
    pub const ALL: [Bundesland; 16] = [
        Bundesland::BadenWuerttemberg,
        Bundesland::Bayern,
        Bundesland::Berlin,
        Bundesland::Brandenburg,
        Bundesland::Bremen,
        Bundesland::Hamburg,
        Bundesland::Hessen,
        Bundesland::MecklenburgVorpommern,
        Bundesland::Niedersachsen,
        Bundesland::NordrheinWestfalen,
        Bundesland::RheinlandPfalz,
        Bundesland::Saarland,
        Bundesland::Sachsen,
        Bundesland::SachsenAnhalt,
        Bundesland::SchleswigHolstein,
        Bundesland::Thueringen,
    ];

    /// 2-letter official ISO code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BadenWuerttemberg => "BW",
            Self::Bayern => "BY",
            Self::Berlin => "BE",
            Self::Brandenburg => "BB",
            Self::Bremen => "HB",
            Self::Hamburg => "HH",
            Self::Hessen => "HE",
            Self::MecklenburgVorpommern => "MV",
            Self::Niedersachsen => "NI",
            Self::NordrheinWestfalen => "NW",
            Self::RheinlandPfalz => "RP",
            Self::Saarland => "SL",
            Self::Sachsen => "SN",
            Self::SachsenAnhalt => "ST",
            Self::SchleswigHolstein => "SH",
            Self::Thueringen => "TH",
        }
    }

    /// Full German state name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::BadenWuerttemberg => "Baden-Württemberg",
            Self::Bayern => "Bayern",
            Self::Berlin => "Berlin",
            Self::Brandenburg => "Brandenburg",
            Self::Bremen => "Bremen",
            Self::Hamburg => "Hamburg",
            Self::Hessen => "Hessen",
            Self::MecklenburgVorpommern => "Mecklenburg-Vorpommern",
            Self::Niedersachsen => "Niedersachsen",
            Self::NordrheinWestfalen => "Nordrhein-Westfalen",
            Self::RheinlandPfalz => "Rheinland-Pfalz",
            Self::Saarland => "Saarland",
            Self::Sachsen => "Sachsen",
            Self::SachsenAnhalt => "Sachsen-Anhalt",
            Self::SchleswigHolstein => "Schleswig-Holstein",
            Self::Thueringen => "Thüringen",
        }
    }

    /// Representative load / industrial hub capital.
    pub fn hub(&self) -> &'static str {
        match self {
            Self::BadenWuerttemberg => "Stuttgart",
            Self::Bayern => "München",
            Self::Berlin => "Berlin",
            Self::Brandenburg => "Potsdam",
            Self::Bremen => "Bremen",
            Self::Hamburg => "Hamburg",
            Self::Hessen => "Frankfurt am Main",
            Self::MecklenburgVorpommern => "Rostock",
            Self::Niedersachsen => "Hannover",
            Self::NordrheinWestfalen => "Düsseldorf / Ruhrgebiet",
            Self::RheinlandPfalz => "Mainz",
            Self::Saarland => "Saarbrücken",
            Self::Sachsen => "Dresden",
            Self::SachsenAnhalt => "Magdeburg",
            Self::SchleswigHolstein => "Kiel",
            Self::Thueringen => "Erfurt",
        }
    }

    /// Primary regional Distribution System Operator (VNB).
    pub fn primary_vnb(&self) -> &'static str {
        match self {
            Self::BadenWuerttemberg => "Netze BW",
            Self::Bayern => "Bayernwerk Netz",
            Self::Berlin => "Stromnetz Berlin",
            Self::Brandenburg => "E.DIS Netz",
            Self::Bremen => "wesernetz",
            Self::Hamburg => "Stromnetz Hamburg",
            Self::Hessen => "Syna / Mainova",
            Self::MecklenburgVorpommern => "WEMAG / E.DIS",
            Self::Niedersachsen => "Avacon Netz",
            Self::NordrheinWestfalen => "Westnetz",
            Self::RheinlandPfalz => "Westnetz / EWR Netz",
            Self::Saarland => "energis-Netzgesellschaft",
            Self::Sachsen => "Mitteldeutsche Netzgesellschaft (MITNETZ)",
            Self::SachsenAnhalt => "MITNETZ STROM / Avacon",
            Self::SchleswigHolstein => "Schleswig-Holstein Netz",
            Self::Thueringen => "Thüringer Energienetze (TEN)",
        }
    }

    /// Latitude of representative weather coordinate.
    pub fn lat(&self) -> f64 {
        match self {
            Self::BadenWuerttemberg => 48.78,
            Self::Bayern => 48.14,
            Self::Berlin => 52.52,
            Self::Brandenburg => 52.40,
            Self::Bremen => 53.08,
            Self::Hamburg => 53.55,
            Self::Hessen => 50.11,
            Self::MecklenburgVorpommern => 54.09,
            Self::Niedersachsen => 52.37,
            Self::NordrheinWestfalen => 51.45,
            Self::RheinlandPfalz => 49.99,
            Self::Saarland => 49.23,
            Self::Sachsen => 51.05,
            Self::SachsenAnhalt => 52.13,
            Self::SchleswigHolstein => 54.32,
            Self::Thueringen => 50.98,
        }
    }

    /// Longitude of representative weather coordinate.
    pub fn lon(&self) -> f64 {
        match self {
            Self::BadenWuerttemberg => 9.18,
            Self::Bayern => 11.58,
            Self::Berlin => 13.40,
            Self::Brandenburg => 13.06,
            Self::Bremen => 8.80,
            Self::Hamburg => 10.00,
            Self::Hessen => 8.68,
            Self::MecklenburgVorpommern => 12.13,
            Self::Niedersachsen => 9.73,
            Self::NordrheinWestfalen => 7.01,
            Self::RheinlandPfalz => 8.27,
            Self::Saarland => 6.99,
            Self::Sachsen => 13.74,
            Self::SachsenAnhalt => 11.63,
            Self::SchleswigHolstein => 10.13,
            Self::Thueringen => 11.03,
        }
    }

    /// DWD monthly baseline temperatures (°C) from Jan to Dec.
    pub fn monthly_temps(&self) -> [f64; 12] {
        match self {
            Self::BadenWuerttemberg => [1.8, 2.7, 6.2, 10.3, 14.7, 18.2, 20.1, 19.6, 15.2, 10.4, 5.6, 2.5],
            Self::Bayern => [-0.5, 0.8, 4.8, 9.2, 13.7, 17.1, 19.0, 18.4, 14.1, 9.3, 4.1, 0.5],
            Self::Berlin => [0.6, 1.8, 5.1, 10.2, 14.8, 18.2, 20.2, 19.8, 15.3, 10.1, 5.1, 1.6],
            Self::Brandenburg => [0.5, 1.6, 5.0, 10.0, 14.6, 18.0, 20.0, 19.5, 15.1, 9.9, 5.0, 1.5],
            Self::Bremen => [2.4, 2.9, 5.6, 9.4, 13.4, 16.2, 18.4, 18.1, 14.8, 10.3, 6.0, 3.2],
            Self::Hamburg => [2.1, 2.6, 5.4, 9.3, 13.5, 16.3, 18.6, 18.3, 14.9, 10.3, 5.9, 2.9],
            Self::Hessen => [2.2, 3.1, 6.7, 10.8, 15.1, 18.5, 20.5, 19.9, 15.6, 10.7, 6.0, 2.9],
            Self::MecklenburgVorpommern => [1.5, 2.0, 4.6, 8.8, 13.1, 16.5, 18.7, 18.5, 14.9, 10.1, 5.6, 2.4],
            Self::Niedersachsen => [2.3, 2.9, 5.9, 9.9, 14.1, 17.0, 19.1, 18.8, 15.1, 10.5, 6.1, 3.0],
            Self::NordrheinWestfalen => [2.8, 3.4, 6.3, 10.0, 14.2, 17.1, 19.1, 18.8, 15.4, 11.1, 6.5, 3.6],
            Self::RheinlandPfalz => [2.3, 3.2, 6.8, 10.9, 15.1, 18.4, 20.4, 19.8, 15.5, 10.6, 6.0, 3.0],
            Self::Saarland => [1.9, 2.8, 6.4, 10.3, 14.4, 17.6, 19.6, 19.1, 15.0, 10.3, 5.7, 2.6],
            Self::Sachsen => [0.4, 1.4, 4.9, 9.6, 14.2, 17.5, 19.5, 19.1, 14.8, 9.8, 4.9, 1.4],
            Self::SachsenAnhalt => [1.1, 1.9, 5.2, 9.8, 14.3, 17.4, 19.5, 19.1, 15.0, 10.1, 5.3, 1.9],
            Self::SchleswigHolstein => [1.8, 2.1, 4.5, 8.4, 12.6, 15.6, 17.9, 17.7, 14.5, 10.1, 5.8, 2.7],
            Self::Thueringen => [0.3, 1.2, 4.7, 9.1, 13.4, 16.6, 18.5, 18.1, 14.1, 9.3, 4.6, 1.2],
        }
    }

    /// Check if hour-of-year falls in the VNB's § 19 StromNEV Hochlastzeitfenster.
    pub fn is_peak_window(&self, h: usize) -> bool {
        let hod = h % 24;
        match self {
            Self::SchleswigHolstein | Self::MecklenburgVorpommern => {
                // High coastal wind: dominant evening peak
                (17..=20).contains(&hod)
            }
            Self::Berlin | Self::Brandenburg => {
                // Metropolitan / eastern grid
                (11..=12).contains(&hod) || (17..=20).contains(&hod)
            }
            _ => {
                // Dual morning & evening industrial peak windows
                (11..=13).contains(&hod) || (17..=19).contains(&hod)
            }
        }
    }

    /// Loose string parser supporting ISO code, full German name, and English aliases.
    pub fn from_str_loose(s: &str) -> Option<Self> {
        let clean = s.trim().to_lowercase().replace('-', "").replace(' ', "").replace('_', "");
        match clean.as_str() {
            "bw" | "badenwürttemberg" | "badenwuerttemberg" | "badewuerttemberg" | "stuttgart" => Some(Self::BadenWuerttemberg),
            "by" | "bayern" | "bavaria" | "münchen" | "muenchen" => Some(Self::Bayern),
            "be" | "berlin" => Some(Self::Berlin),
            "bb" | "brandenburg" | "potsdam" => Some(Self::Brandenburg),
            "hb" | "bremen" => Some(Self::Bremen),
            "hh" | "hamburg" => Some(Self::Hamburg),
            "he" | "hessen" | "hesse" | "frankfurt" => Some(Self::Hessen),
            "mv" | "mecklenburgvorpommern" | "mecklenburg" | "rostock" => Some(Self::MecklenburgVorpommern),
            "ni" | "niedersachsen" | "lowersaxony" | "hannover" => Some(Self::Niedersachsen),
            "nw" | "nrw" | "nordrheinwestfalen" | "northrhinewestphalia" | "düsseldorf" | "duesseldorf" | "essen" | "köln" | "koeln" => Some(Self::NordrheinWestfalen),
            "rp" | "rheinlandpfalz" | "rhinelandpalatinate" | "mainz" => Some(Self::RheinlandPfalz),
            "sl" | "saarland" | "saarbrücken" | "saarbruecken" => Some(Self::Saarland),
            "sn" | "sachsen" | "saxony" | "dresden" | "leipzig" => Some(Self::Sachsen),
            "st" | "sachsenanhalt" | "saxonyanhalt" | "magdeburg" => Some(Self::SachsenAnhalt),
            "sh" | "schleswigholstein" | "kiel" => Some(Self::SchleswigHolstein),
            "th" | "thüringen" | "thueringen" | "thuringia" | "erfurt" => Some(Self::Thueringen),
            _ => None,
        }
    }
}

impl Default for Bundesland {
    fn default() -> Self {
        Self::NordrheinWestfalen
    }
}

impl fmt::Display for Bundesland {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name(), self.code())
    }
}

impl FromStr for Bundesland {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str_loose(s).ok_or_else(|| {
            format!(
                "Unbekanntes Bundesland '{}'. Gültige Codes: BW, BY, BE, BB, HB, HH, HE, MV, NI, NW, RP, SL, SN, ST, SH, TH",
                s
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_16_states_exist_and_are_unique() {
        assert_eq!(Bundesland::ALL.len(), 16);
        let mut codes = std::collections::HashSet::new();
        for b in Bundesland::ALL {
            assert!(codes.insert(b.code()));
            assert!(!b.name().is_empty());
            assert!(!b.primary_vnb().is_empty());
            assert!(b.lat() >= 47.0 && b.lat() <= 55.0);
            assert!(b.lon() >= 5.0 && b.lon() <= 15.5);
            assert_eq!(b.monthly_temps().len(), 12);
        }
    }

    #[test]
    fn test_loose_parsing() {
        assert_eq!(Bundesland::from_str_loose("by"), Some(Bundesland::Bayern));
        assert_eq!(Bundesland::from_str_loose("Bayern"), Some(Bundesland::Bayern));
        assert_eq!(Bundesland::from_str_loose("Bavaria"), Some(Bundesland::Bayern));
        assert_eq!(Bundesland::from_str_loose("nrw"), Some(Bundesland::NordrheinWestfalen));
        assert_eq!(Bundesland::from_str_loose("NW"), Some(Bundesland::NordrheinWestfalen));
        assert_eq!(Bundesland::from_str_loose("Baden-Württemberg"), Some(Bundesland::BadenWuerttemberg));
        assert_eq!(Bundesland::from_str_loose("sh"), Some(Bundesland::SchleswigHolstein));
        assert_eq!(Bundesland::from_str_loose("invalid_state"), None);
    }
}
