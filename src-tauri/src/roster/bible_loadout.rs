//! lostark.bible character pages embed `loadouts: [{ classification: "...", items: [...], gems: [...], ... }]`.
//! Multiple raid-related tabs exist (Estimated / Raid / Current). We always prefer the same source as the site UI:
//! 1. **Estimated Raid Loadout** (if present in payload)
//! 2. else **Raid Loadout**
//! 3. else **Current Loadout (Raid)** (`most_recent_raid` in JSON samples)
//!
//! Chaos dungeon loadouts (e.g. `most_recent_chaos_dungeon`) are ignored for this selection.

use serde_json::Value;

/// Synonyms per UI tab, inner slices are OR-matched within the same priority tier.
/// Extend when lostark.bible adds new `classification` strings.
/// Priority groups for selecting the preferred raid loadout.
///
/// Adjusted to prefer the server-provided merged/estimated raid when available
/// and otherwise fall back to the most recent raid loadout. Chaos classifications
/// are explicitly ignored elsewhere.
static RAID_LOADOUT_PRIORITY_GROUPS: &[&[&str]] = &[
    // Highest priority: merged/estimated raid provided by the site
    &["raid_merged"],
    // Fallback: most recent/current raid
    &["most_recent_raid"],
];

/// True if this loadout is clearly the chaos tab, not raid progression.
fn is_chaos_dungeon_classification(classification: &str) -> bool {
    let c = classification.to_ascii_lowercase();
    c.contains("chaos")
}

/// Returns the loadout object to use for raid gear / raid gems / engravings derived from that tab.
pub fn pick_preferred_raid_loadout(loadouts: &[Value]) -> Option<&Value> {
    if loadouts.is_empty() {
        return None;
    }

    for group in RAID_LOADOUT_PRIORITY_GROUPS {
        for key in *group {
            if let Some(found) = loadouts.iter().find(|lo| {
                lo.get("classification")
                    .and_then(|c| c.as_str())
                    .map(|cl| cl == *key && !is_chaos_dungeon_classification(cl))
                    .unwrap_or(false)
            }) {
                return Some(found);
            }
        }
    }

    // Fallback: first non-chaos entry (e.g. unknown new classification, still raid-ish)
    loadouts
        .iter()
        .find(|lo| {
            lo.get("classification")
                .and_then(|c| c.as_str())
                .map(|cl| !is_chaos_dungeon_classification(cl))
                .unwrap_or(true)
        })
        .or_else(|| loadouts.first())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prefers_raid_merged_over_most_recent() {
        let v = vec![
            json!({"classification": "most_recent_chaos_dungeon"}),
            json!({"classification": "most_recent_raid", "id": 1}),
            json!({"classification": "raid_merged", "id": 2}),
        ];
        assert_eq!(pick_preferred_raid_loadout(&v).unwrap()["id"], 2);
    }

    #[test]
    fn prefers_most_recent_when_no_merged() {
        let v = vec![
            json!({"classification": "most_recent_chaos_dungeon"}),
            json!({"classification": "most_recent_raid", "id": 3}),
        ];
        assert_eq!(pick_preferred_raid_loadout(&v).unwrap()["id"], 3);
    }

    #[test]
    fn ignores_chaos_and_falls_back_to_first_non_chaos() {
        let v = vec![
            json!({"classification": "most_recent_chaos_dungeon", "id": 0}),
            json!({"classification": "unknown_loadout", "id": 5}),
        ];
        // No priority match; should pick the first non-chaos entry (id 5)
        assert_eq!(pick_preferred_raid_loadout(&v).unwrap()["id"], 5);
    }
}
