use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub general: GeneralSettings,
    pub system: SystemSettings,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SystemSettings {
    pub encounters_db_path: Option<String>,
    pub lost_ark_exe_path: Option<String>,
    pub start_with_windows: bool,
    pub start_with_lost_ark: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            extra: Map::new(),
        }
    }
}

impl Default for SystemSettings {
    fn default() -> Self {
        Self {
            encounters_db_path: None,
            lost_ark_exe_path: None,
            start_with_windows: false,
            start_with_lost_ark: false,
            extra: Map::new(),
        }
    }
}
