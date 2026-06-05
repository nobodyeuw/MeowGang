use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy)]
pub struct GemSkillLabel {
    pub name: &'static str,
    pub icon: &'static str,
}

pub fn get_gem_skill_label(id: i64) -> Option<GemSkillLabel> {
    GEM_SKILL_GROUPS.get(&id).copied().or_else(|| SKILLS.get(&id).copied())
}

static GEM_SKILL_GROUPS: LazyLock<HashMap<i64, GemSkillLabel>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Source: LOA Logs meter-data/GemSkillGroup.json.
    // Keep grouped gem labels here because lostark.bible gem effects can point
    // at gem skill groups, not only concrete skill ids.
    m.insert(
        53000,
        GemSkillLabel {
            name: "Barrage Skill",
            icon: "bs_skill_01_8.png",
        },
    );
    m.insert(
        530000,
        GemSkillLabel {
            name: "Barrage Skill",
            icon: "bs_skill_01_8.png",
        },
    );
    m.insert(
        53001,
        GemSkillLabel {
            name: "Barrage Skill",
            icon: "bs_skill_01_8.png",
        },
    );

    m
});

static SKILLS: LazyLock<HashMap<i64, GemSkillLabel>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Source: LOA Logs meter-data/Skill.json. Start with Artillerist/Barrage
    // because Vaanyar is the first validation character for the planner.
    m.insert(
        30050,
        GemSkillLabel {
            name: "Enhanced Shell",
            icon: "bs_skill_01_0.png",
        },
    );
    m.insert(
        30100,
        GemSkillLabel {
            name: "Howitzer",
            icon: "bs_skill_01_1.png",
        },
    );
    m.insert(
        30060,
        GemSkillLabel {
            name: "Pressurized Heatbomb",
            icon: "bs_skill_01_5.png",
        },
    );
    m.insert(
        30110,
        GemSkillLabel {
            name: "Air Raid",
            icon: "bs_skill_01_15.png",
        },
    );
    m.insert(
        30120,
        GemSkillLabel {
            name: "Flamethrower",
            icon: "bs_skill_01_9.png",
        },
    );
    m.insert(
        30160,
        GemSkillLabel {
            name: "Flamethrower",
            icon: "bs_skill_01_9.png",
        },
    );
    m.insert(
        30180,
        GemSkillLabel {
            name: "Napalm Shot",
            icon: "bs_skill_01_11.png",
        },
    );
    m.insert(
        30200,
        GemSkillLabel {
            name: "Gravity Explosion",
            icon: "bs_skill_01_12.png",
        },
    );
    m.insert(
        30220,
        GemSkillLabel {
            name: "Homing Barrage",
            icon: "bs_skill_01_8.png",
        },
    );
    m.insert(
        30230,
        GemSkillLabel {
            name: "Air Raid",
            icon: "bs_skill_01_15.png",
        },
    );
    m.insert(
        30240,
        GemSkillLabel {
            name: "Homing Barrage",
            icon: "bs_skill_01_8.png",
        },
    );
    m.insert(
        30250,
        GemSkillLabel {
            name: "Missile Barrage",
            icon: "bs_skill_01_16.png",
        },
    );
    m.insert(
        30260,
        GemSkillLabel {
            name: "Barrage: Howitzer",
            icon: "bs_skill_01_18.png",
        },
    );
    m.insert(
        30270,
        GemSkillLabel {
            name: "Barrage: Focus Fire",
            icon: "bs_skill_01_19.png",
        },
    );
    m.insert(
        30290,
        GemSkillLabel {
            name: "Barrage: Energy Cannon",
            icon: "bs_skill_01_20.png",
        },
    );
    m.insert(
        30390,
        GemSkillLabel {
            name: "Barrage: Steel Rain",
            icon: "bs_skill_01_31.png",
        },
    );
    m.insert(
        30392,
        GemSkillLabel {
            name: "Barrage: A.C.T",
            icon: "bs_skill_01_33.png",
        },
    );
    m.insert(
        30393,
        GemSkillLabel {
            name: "Barrage: A.C.T",
            icon: "bs_skill_01_33.png",
        },
    );

    m
});
