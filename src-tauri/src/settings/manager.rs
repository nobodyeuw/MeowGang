use anyhow::Result;
use std::{fs::File, path::PathBuf};

use crate::settings::Settings;

pub struct SettingsManager(PathBuf);

impl SettingsManager {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Self(path))
    }

    pub fn read(&self) -> Result<Option<Settings>> {
        if !self.0.exists() {
            return Ok(None);
        }

        let reader = File::open(&self.0)?;
        let settings: Settings = serde_json::from_reader(reader)?;
        Ok(Some(settings))
    }

    pub fn save(&self, settings: &Settings) -> Result<()> {
        let writer = File::create(&self.0)?;
        serde_json::to_writer_pretty(writer, settings)?;

        Ok(())
    }

    pub fn get_default(&self) -> Settings {
        Settings::default()
    }

    pub fn ensure_exists(&self) -> Result<Settings> {
        match self.read()? {
            Some(settings) => Ok(settings),
            None => {
                let default_settings = self.get_default();
                self.save(&default_settings)?;
                Ok(default_settings)
            }
        }
    }
}
