use std::collections::HashMap;
use std::sync::LazyLock;

/// Engraving ID to name mapping
/// Source: lostark.bible engraving data
pub fn get_engraving_name(id: i64) -> Option<&'static str> {
    ENGRAVINGS.get(&id).copied()
}

/// Gem ID to name mapping
/// Source: lostark.bible gem data
pub fn get_gem_name(id: i64) -> Option<&'static str> {
    GEMS.get(&id).copied()
}

// Engraving mappings (ID -> Name)
// Note: Some IDs have duplicates with different values, we use the most common ones
static ENGRAVINGS: LazyLock<HashMap<i64, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    
    // Core engravings
    m.insert(107, "Disrespect");
    m.insert(109, "Spirit Absorption");
    m.insert(110, "Ether Predator");
    m.insert(111, "Stabilized Status");
    m.insert(112, "Master of Slashes");
    m.insert(118, "Grudge");
    m.insert(119, "Invincible Evasion");
    m.insert(121, "Super Charge");
    m.insert(123, "Strong Will");
    m.insert(125, "Mayhem");
    m.insert(127, "Esoteric Skill Enhancement");
    m.insert(129, "Enhanced Weapon");
    m.insert(130, "Firepower Enhancement");
    m.insert(133, "Balanced Defense");
    m.insert(134, "Drops of Ether");
    m.insert(140, "Crisis Evasion");
    m.insert(141, "Keen Blunt Weapon");
    m.insert(142, "Vital Point Hit");
    m.insert(157, "Master of Piercing");
    m.insert(158, "Master of Destruction");
    m.insert(167, "Max MP Increase");
    m.insert(168, "MP Efficiency Increase");
    m.insert(188, "Berserker Technique");
    m.insert(189, "First Intention");
    m.insert(190, "Ultimate Skill: Taijutsu");
    m.insert(191, "Shock Training");
    m.insert(192, "Pistoleer");
    m.insert(193, "Barrage Enhancement");
    m.insert(194, "True Courage");
    m.insert(195, "Desperate Salvation");
    m.insert(196, "Rage Hammer");
    m.insert(197, "Gravity Training");
    m.insert(198, "Master Summoner");
    m.insert(199, "Communication Overflow");
    m.insert(200, "Grace of the Empress");
    m.insert(201, "Order of the Emperor");
    m.insert(202, "Master of Escape");
    m.insert(206, "Telescope");
    m.insert(224, "Combat Readiness");
    m.insert(225, "Lone Knight");
    m.insert(235, "Fortitude");
    m.insert(236, "Crushing Fist");
    m.insert(237, "Shield Piercing");
    m.insert(238, "Master's Tenacity");
    m.insert(239, "Divine Protection");
    m.insert(240, "Heavy Armor");
    m.insert(241, "Explosive Expert");
    m.insert(242, "Enhanced Shield");
    m.insert(243, "Necromancy");
    m.insert(244, "Preemptive Strike");
    m.insert(245, "Broken Bone");
    m.insert(246, "Lightning Fury");
    m.insert(247, "Cursed Doll");
    m.insert(248, "Contender");
    m.insert(249, "Ambush Master");
    m.insert(251, "Magick Stream");
    m.insert(253, "Barricade");
    m.insert(254, "Raid Captain");
    m.insert(255, "Awakening");
    m.insert(256, "Energy Overflow");
    m.insert(257, "Robust Spirit");
    m.insert(258, "Loyal Companion");
    m.insert(259, "Death Strike");
    m.insert(114, "Twinkle Twinkle");
    m.insert(116, "Servant");
    m.insert(207, "Dynamite");
    m.insert(211, "Master Net Caster");
    m.insert(213, "Giant Tree");
    m.insert(214, "Sapling");
    m.insert(215, "4-Leaf Clover");
    m.insert(217, "Entomologist");
    m.insert(219, "Butcher");
    m.insert(221, "Delicate Brush");
    
    // Class-specific engravings
    m.insert(276, "Pinnacle");
    m.insert(277, "Control");
    m.insert(278, "Remaining Energy");
    m.insert(279, "Surge");
    m.insert(280, "Perfect Suppression");
    m.insert(281, "Demonic Impulse");
    m.insert(282, "Judgment");
    m.insert(283, "Blessed Aura");
    m.insert(284, "Arthetinean Skill");
    m.insert(285, "Evolutionary Legacy");
    m.insert(286, "Hunger");
    m.insert(287, "Lunar Voice");
    m.insert(288, "Master Brawler");
    m.insert(289, "Peacemaker");
    m.insert(290, "Time to Hunt");
    m.insert(291, "Deathblow");
    m.insert(292, "Esoteric Flurry");
    m.insert(293, "Igniter");
    m.insert(294, "Reflux");
    m.insert(295, "Mass Increase");
    m.insert(296, "Propulsion");
    m.insert(297, "Hit Master");
    m.insert(298, "Sight Focus");
    m.insert(299, "Adrenaline");
    m.insert(300, "All-Out Attack");
    m.insert(301, "Expert");
    m.insert(302, "Emergency Rescue");
    m.insert(303, "Precise Dagger");
    m.insert(305, "Recurrence");
    m.insert(306, "Full Bloom");
    m.insert(307, "Wind Fury");
    m.insert(308, "Drizzle");
    m.insert(309, "Predator");
    m.insert(310, "Punisher");
    m.insert(311, "Full Moon Harvester");
    m.insert(312, "Night's Edge");
    m.insert(314, "Brawl King Storm");
    m.insert(315, "Asura's Path");
    
    // Negative engravings
    m.insert(800, "Atk. Power Reduction");
    m.insert(801, "Defense Reduction");
    m.insert(802, "Atk. Speed Reduction");
    m.insert(803, "Move Speed Reduction");
    
    // Duplicate IDs (alternate versions)
    m.insert(1111, "Stabilized Status");
    m.insert(1118, "Grudge");
    m.insert(1141, "Keen Blunt Weapon");
    m.insert(1299, "Adrenaline");
    m.insert(1300, "All-Out Attack");
    m.insert(1301, "Expert");
    m.insert(1302, "Emergency Rescue");
    m.insert(1303, "Precise Dagger");
    m.insert(1295, "Mass Increase");
    m.insert(1296, "Propulsion");
    m.insert(1297, "Hit Master");
    m.insert(1298, "Sight Focus");
    m.insert(1121, "Super Charge");
    m.insert(1123, "Strong Will");
    m.insert(1134, "Drops of Ether");
    m.insert(1140, "Crisis Evasion");
    m.insert(1142, "Vital Point Hit");
    m.insert(1167, "Max MP Increase");
    m.insert(1168, "MP Efficiency Increase");
    m.insert(1202, "Master of Escape");
    m.insert(1235, "Fortitude");
    m.insert(1236, "Crushing Fist");
    m.insert(1237, "Shield Piercing");
    m.insert(1238, "Master's Tenacity");
    m.insert(1239, "Divine Protection");
    m.insert(1240, "Heavy Armor");
    m.insert(1241, "Explosive Expert");
    m.insert(1242, "Enhanced Shield");
    m.insert(1243, "Necromancy");
    m.insert(1244, "Preemptive Strike");
    m.insert(1245, "Broken Bone");
    m.insert(1246, "Lightning Fury");
    m.insert(1247, "Cursed Doll");
    m.insert(1248, "Contender");
    m.insert(1249, "Ambush Master");
    m.insert(1251, "Magick Stream");
    m.insert(1253, "Barricade");
    m.insert(1254, "Raid Captain");
    m.insert(1255, "Awakening");
    m.insert(1288, "Master Brawler");
    m.insert(1800, "Atk. Power Reduction");
    m.insert(1801, "Defense Reduction");
    m.insert(1802, "Atk. Speed Reduction");
    m.insert(1803, "Move Speed Reduction");
    
    // Life skill engravings
    m.insert(260, "Increase Mining Tools");
    m.insert(261, "Relentless Miner");
    m.insert(262, "Bomb Enthusiast");
    m.insert(263, "Fishing Tools Increase");
    m.insert(264, "Double Points");
    m.insert(265, "Golden Bait");
    m.insert(266, "Logging Tool Rank Boost");
    m.insert(267, "Rapid Kick");
    m.insert(268, "Increase Foraging Tools");
    m.insert(269, "Deceptive Foraging");
    m.insert(270, "Increase Hunting Tools");
    m.insert(271, "Deadly Poison");
    m.insert(272, "Golden Rabbit");
    m.insert(273, "Increase Excavation Tools");
    m.insert(274, "Loot Hunter");
    m.insert(275, "Master Tamer");
    
    m
});

// Gem mappings (ID -> Name)
static GEMS: LazyLock<HashMap<i64, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    
    // Apprentice Gems
    m.insert(65001010, "Lv. 1 Apprentice's Gem (Bound)");
    m.insert(65001020, "Lv. 2 Apprentice's Gem (Bound)");
    
    // Azure Gems
    m.insert(65011010, "Level 1 Azure Gem");
    m.insert(65011020, "Level 2 Azure Gem");
    m.insert(65011030, "Level 3 Azure Gem");
    m.insert(65011040, "Level 4 Azure Gem");
    m.insert(65011050, "Level 5 Azure Gem");
    m.insert(65011060, "Level 6 Azure Gem");
    m.insert(65011070, "Level 7 Azure Gem");
    m.insert(65011080, "Level 8 Azure Gem");
    m.insert(65011090, "Level 9 Azure Gem");
    m.insert(65011100, "Level 10 Azure Gem");
    
    // Farsea Gems
    m.insert(65012010, "Level 1 Farsea Gem");
    m.insert(65012020, "Level 2 Farsea Gem");
    m.insert(65012030, "Level 3 Farsea Gem");
    m.insert(65012040, "Level 4 Farsea Gem");
    m.insert(65012050, "Level 5 Farsea Gem");
    m.insert(65012060, "Level 6 Farsea Gem");
    m.insert(65012070, "Level 7 Farsea Gem");
    m.insert(65012080, "Level 8 Farsea Gem");
    m.insert(65012090, "Level 9 Farsea Gem");
    m.insert(65012100, "Level 10 Farsea Gem");
    
    // Annihilation Gems
    m.insert(65021010, "Lv. 1 Annihilation Gem");
    m.insert(65021020, "Lv. 2 Annihilation Gem");
    m.insert(65021030, "Lv. 3 Annihilation Gem");
    m.insert(65021040, "Lv. 4 Annihilation Gem");
    m.insert(65021050, "Lv. 5 Annihilation Gem");
    m.insert(65021060, "Lv. 6 Annihilation Gem");
    m.insert(65021070, "Lv. 7 Annihilation Gem");
    m.insert(65021080, "Lv. 8 Annihilation Gem");
    m.insert(65021090, "Lv. 9 Annihilation Gem");
    m.insert(65021100, "Lv. 10 Annihilation Gem");
    
    // Crimson Flame Gems
    m.insert(65022010, "Lv. 1 Crimson Flame Gem");
    m.insert(65022020, "Lv. 2 Crimson Flame Gem");
    m.insert(65022030, "Lv. 3 Crimson Flame Gem");
    m.insert(65022040, "Lv. 4 Crimson Flame Gem");
    m.insert(65022050, "Lv. 5 Crimson Flame Gem");
    m.insert(65022060, "Lv. 6 Crimson Flame Gem");
    m.insert(65022070, "Lv. 7 Crimson Flame Gem");
    m.insert(65022080, "Lv. 8 Crimson Flame Gem");
    m.insert(65022090, "Lv. 9 Crimson Flame Gem");
    m.insert(65022100, "Lv. 10 Crimson Flame Gem");
    
    // Doomfire Gems
    m.insert(65031010, "Lv. 1 Doomfire Gem");
    m.insert(65031020, "Lv. 2 Doomfire Gem");
    m.insert(65031030, "Lv. 3 Doomfire Gem");
    m.insert(65031040, "Lv. 4 Doomfire Gem");
    m.insert(65031050, "Lv. 5 Doomfire Gem");
    m.insert(65031060, "Lv. 6 Doomfire Gem");
    m.insert(65031061, "Lv. 6 Doomfire Gem (Bound)");
    m.insert(65031070, "Lv. 7 Doomfire Gem");
    m.insert(65031080, "Lv. 8 Doomfire Gem");
    m.insert(65031090, "Lv. 9 Doomfire Gem");
    m.insert(65031100, "Lv. 10 Doomfire Gem");
    
    // Blazing Gems
    m.insert(65032010, "Lv. 1 Blazing Gem");
    m.insert(65032020, "Lv. 2 Blazing Gem");
    m.insert(65032030, "Lv. 3 Blazing Gem");
    m.insert(65032040, "Lv. 4 Blazing Gem");
    m.insert(65032050, "Lv. 5 Blazing Gem");
    m.insert(65032060, "Lv. 6 Blazing Gem");
    m.insert(65032061, "Lv. 6 Blazing Gem (Bound)");
    m.insert(65032070, "Lv. 7 Blazing Gem");
    m.insert(65032080, "Lv. 8 Blazing Gem");
    m.insert(65032090, "Lv. 9 Blazing Gem");
    m.insert(65032100, "Lv. 10 Blazing Gem");
    
    // Brilliant Gems
    m.insert(65041010, "Lv. 1 Brilliant Gem");
    m.insert(65041020, "Lv. 2 Brilliant Gem");
    m.insert(65041030, "Lv. 3 Brilliant Gem");
    m.insert(65041040, "Lv. 4 Brilliant Gem");
    m.insert(65041050, "Lv. 5 Brilliant Gem");
    m.insert(65041051, "Lv. 5 Brilliant Gem (Bound)");
    m.insert(65041060, "Lv. 6 Brilliant Gem");
    m.insert(65041061, "Lv. 6 Brilliant Gem (Bound)");
    m.insert(65041070, "Lv. 7 Brilliant Gem");
    m.insert(65041071, "Lv. 7 Brilliant Gem (Bound)");
    m.insert(65041072, "Lv. 7 Brilliant Gem (Bound)");
    m.insert(65041073, "Lv. 7 Brilliant Gem (Bound)");
    m.insert(65041080, "Lv. 8 Brilliant Gem");
    m.insert(65041082, "Lv. 8 Brilliant Gem (Bound)");
    m.insert(65041090, "Lv. 9 Brilliant Gem");
    m.insert(65041100, "Lv. 10 Brilliant Gem");
    
    // Brilliant Gems (alternate)
    m.insert(65042010, "Lv. 1 Brilliant Gem");
    m.insert(65042020, "Lv. 2 Brilliant Gem");
    m.insert(65042030, "Lv. 3 Brilliant Gem");
    m.insert(65042040, "Lv. 4 Brilliant Gem");
    m.insert(65042050, "Lv. 5 Brilliant Gem");
    m.insert(65042060, "Lv. 6 Brilliant Gem");
    m.insert(65042070, "Lv. 7 Brilliant Gem");
    m.insert(65042080, "Lv. 8 Brilliant Gem");
    m.insert(65042090, "Lv. 9 Brilliant Gem");
    m.insert(65042100, "Lv. 10 Brilliant Gem");
    
    // Bound Annihilation Gems (many variants)
    m.insert(65091010, "Lv. 1 Annihilation Gem (Bound)");
    m.insert(65091020, "Lv. 2 Annihilation Gem (Bound)");
    m.insert(65091030, "Lv. 3 Annihilation Gem (Bound)");
    m.insert(65091040, "Lv. 4 Annihilation Gem (Bound)");
    m.insert(65091050, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091051, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091052, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091053, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091054, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091055, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091056, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091057, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091058, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091059, "Lv. 5 Annihilation Gem (Bound)");
    m.insert(65091060, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091061, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091062, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091063, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091064, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091065, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091066, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091067, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091068, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091069, "Lv. 6 Annihilation Gem (Bound)");
    m.insert(65091070, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091071, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091072, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091073, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091074, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091075, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091076, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091077, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091078, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091079, "Lv. 7 Annihilation Gem (Bound)");
    m.insert(65091080, "Lv. 8 Annihilation Gem (Bound)");
    m.insert(65091090, "Lv. 9 Annihilation Gem (Bound)");
    m.insert(65091100, "Lv. 10 Annihilation Gem (Bound)");
    
    // Additional bound gem variants (many duplicates with same name)
    // Adding just the unique ones for now
    m.insert(65093021, "Lv. 9 Annihilation Gem (Bound)");
    m.insert(65093007, "Lv. 9 Crimson Flame Gem (Bound)");
    m.insert(65093010, "Lv. 7 Doomfire Gem (Bound)");
    m.insert(65093015, "Lv. 7 Blazing Gem (Bound)");
    m.insert(65093019, "Lv. 7 Brilliant Gem (Bound)");
    m.insert(65093020, "Lv. 7 Brilliant Gem (Bound)");
    m.insert(65093021, "Lv. 8 Brilliant Gem");
    
    m
});
