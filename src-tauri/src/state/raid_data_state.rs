use std::sync::RwLock;
use crate::database::data_manager::{Raid, RaidGate};

/// State for storing raid data in RAM (frontend-driven)
/// This allows the backend to access raid gold values without hardcoding
pub struct RaidDataState {
    /// RwLock allows concurrent reads but safe writes
    pub raids: RwLock<Vec<Raid>>,
}

impl RaidDataState {
    /// Create a new empty raid data state
    pub fn new() -> Self {
        Self {
            raids: RwLock::new(Vec::new()),
        }
    }

    /// Update the raid data state
    pub fn update_raids(&self, raids: Vec<Raid>) -> Result<(), String> {
        let mut lock = self.raids.write().map_err(|_| "State Lock Error")?;
        *lock = raids;
        Ok(())
    }

    /// Get a read-only reference to the raid data
    pub fn get_raids(&self) -> Result<Vec<Raid>, String> {
        let lock = self.raids.read().map_err(|_| "State Read Error")?;
        Ok(lock.clone())
    }

    /// Find a specific raid by id and difficulty (case-insensitive)
    pub fn find_raid(&self, raid_id: &str, difficulty: &str) -> Option<Raid> {
        let lock = self.raids.read().ok()?;
        lock.iter()
            .find(|r| r.id == raid_id && r.difficulty.to_lowercase() == difficulty.to_lowercase())
            .cloned()
    }

    /// Find a specific gate within a raid
    pub fn find_gate(&self, raid_id: &str, difficulty: &str, gate: &str) -> Option<RaidGate> {
        let raid = self.find_raid(raid_id, difficulty)?;
        raid.gates.into_iter()
            .find(|g| g.gate == gate)
    }
}
