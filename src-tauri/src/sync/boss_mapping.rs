use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct EncounterMapping {
    pub content_id: String,
    pub gate: u8,
    pub boss_names: Vec<String>,
}

pub struct BossMapper {
    mappings: HashMap<String, Vec<EncounterMapping>>,
}

impl BossMapper {
    pub fn new() -> Self {
        let mut mapper = Self {
            mappings: HashMap::new(),
        };
        mapper.initialize_mappings();
        mapper
    }

    fn initialize_mappings(&mut self) {
        // overture_echidna
        self.add_mapping(
            "overture_echidna",
            1,
            vec![
                "Red Doom Narkiel".to_string(),
                "Agris".to_string(),
            ],
        );
        self.add_mapping(
            "overture_echidna",
            2,
            vec![
                "Echidna".to_string(),
                "Covetous Master Echidna".to_string(),
                "Desire in Full Bloom, Echidna".to_string(),
                "Alcaone, the Twisted Venom".to_string(),
                "Agris, the Devouring Bog".to_string(),
            ],
        );

        // behemoth
        self.add_mapping(
            "behemoth",
            1,
            vec![
                "Behemoth, the Storm Commander".to_string()
            ],
        );
        self.add_mapping(
            "behemoth",
            2,
            vec!["Behemoth, Cruel Storm Slayer".to_string()],
        );

        // act_1_aegir
        self.add_mapping(
            "act_1_aegir",
            1,
            vec![
                "Akkan, Lord of Death".to_string(),
                "Abyss Monarch Aegir".to_string(),
            ],
        );
        self.add_mapping(
            "act_1_aegir",
            2,
            vec![
                "Aegir, the Oppressor".to_string(),
                "Pulsating Giant's Heart".to_string(),
            ],
        );

        // act_2_brelshaza
        self.add_mapping(
            "act_2_brelshaza",
            1,
            vec!["Narok the Butcher".to_string()],
        );
        self.add_mapping(
            "act_2_brelshaza",
            2,
            vec![
                "Phantom Legion Commander Brelshaza".to_string(),
                "Phantom Manifester Brelshaza".to_string(),
            ],
        );

        // act_3_mordum
        self.add_mapping(
            "act_3_mordum",
            1,
            vec![
                "Thaemine, Master of Darkness".to_string(),
                "Infernas".to_string(),
            ],
        );
        self.add_mapping(
            "act_3_mordum",
            2,
            vec!["Blossoming Fear, Naitreya".to_string()],
        );
        self.add_mapping(
            "act_3_mordum",
            3,
            vec![
                "Mordum, the Abyssal Punisher".to_string(),
                "Mordum's Hammer".to_string(),
                "Flash of Punishment".to_string(),
            ],
        );

        // act_4_armoche
        self.add_mapping(
            "act_4_armoche",
            1,
            vec![
                "Act 4: Covetous Master Echidna".to_string(),
                "Brelshaza, Ember in the Ashes".to_string(),
            ],
        );
        self.add_mapping(
            "act_4_armoche",
            2,
            vec!["Armoche, Sentinel of the Abyss".to_string()],
        );

        // denouement_final_day
        self.add_mapping(
            "denouement_final_day",
            1,
            vec![
                "Abyss Lord Kazeros".to_string(),
                "Abyssal Afterimage".to_string(),
            ],
        );
        self.add_mapping(
            "denouement_final_day",
            2,
            vec![
                "Archdemon Kazeros".to_string(),
                "Death Incarnate Kazeros".to_string(),
            ],
        );

        // Serca
        self.add_mapping(
            "shadow_serca",
            1,
            vec!["Witch of Agony, Serca".to_string()],
        );
        self.add_mapping(
            "shadow_serca",
            2,
            vec!["Corvus Tul Rak".to_string()],
        );
    }

    fn add_mapping(&mut self, content_id: &str, gate: u8, boss_names: Vec<String>) {
        let mapping = EncounterMapping {
            content_id: content_id.to_string(),
            gate,
            boss_names,
        };

        for boss_name in &mapping.boss_names {
            self.mappings
                .entry(boss_name.clone())
                .or_insert_with(Vec::new)
                .push(mapping.clone());
        }
    }

    pub fn map_boss_to_encounter(&self, boss_name: &str) -> Option<&EncounterMapping> {
        // Direct match first
        if let Some(mappings) = self.mappings.get(boss_name) {
            return Some(&mappings[0]); // Return first match
        }

        // Partial match for variations
        for (mapped_boss, mappings) in &self.mappings {
            if boss_name.contains(mapped_boss) || mapped_boss.contains(boss_name) {
                return Some(&mappings[0]);
            }
        }

        None
    }

    pub fn normalize_difficulty(&self, difficulty: &str) -> String {
        match difficulty.to_lowercase().trim() {
            "" => "normal".to_string(),
            d if d.contains("hard") => "hard".to_string(),
            d if d.contains("normal") => "normal".to_string(),
            d if d.contains("inferno") => "hard".to_string(),
            d if d.contains("nightmare") => "nightmare".to_string(),
            d => d.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boss_mapping() {
        let mapper = BossMapper::new();
        
        // Test direct matches
        let echidna = mapper.map_boss_to_encounter("Echidna");
        assert!(echidna.is_some());
        assert_eq!(echidna.unwrap().content_id, "overture_echidna");
        assert_eq!(echidna.unwrap().gate, 2);

        let thaemine = mapper.map_boss_to_encounter("Thaemine, Master of Darkness");
        assert!(thaemine.is_some());
        assert_eq!(thaemine.unwrap().content_id, "act_3_mordum");
        assert_eq!(thaemine.unwrap().gate, 1);
    }

    #[test]
    fn test_difficulty_normalization() {
        let mapper = BossMapper::new();
        
        assert_eq!(mapper.normalize_difficulty(""), "normal");
        assert_eq!(mapper.normalize_difficulty("Normal"), "normal");
        assert_eq!(mapper.normalize_difficulty("Hard"), "hard");
        assert_eq!(mapper.normalize_difficulty("Inferno"), "hard");
    }
}
