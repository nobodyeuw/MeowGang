// Source of truth for tracked raid definitions, gate rewards, difficulty names, and min ilvl rules.
// Encounter-log name aliases belong in encounters.ts so parser mappings do not leak into raid config.
export interface RaidGate {
  gate: string;
  minIlvl: number;
  tradableGold: number;
  boundGold: number;
  boxPrice: number;
}

export interface Raid {
  id: string;
  name: string;
  difficulty: "Solo" | "Normal" | "Hard" | "Nightmare" | "Level 1" | "Level 2" | "Level 3";
  gates: RaidGate[];
}

export const RAIDS: Raid[] = [
  {
    id: "overture_echidna",
    name: "Echidna",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1620, tradableGold: 0, boundGold: 1900, boxPrice: 310 },
      { gate: "Gate 2", minIlvl: 1620, tradableGold: 0, boundGold: 4200, boxPrice: 700 }
    ]
  },
  {
    id: "act_1_aegir",
    name: "Aegir",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1660, tradableGold: 1750, boundGold: 1750, boxPrice: 750 },
      { gate: "Gate 2", minIlvl: 1660, tradableGold: 4000, boundGold: 4000, boxPrice: 1780 }
    ]
  },
  {
    id: "act_1_aegir",
    name: "Aegir",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1660, tradableGold: 1750, boundGold: 1750, boxPrice: 750 },
      { gate: "Gate 2", minIlvl: 1660, tradableGold: 4000, boundGold: 4000, boxPrice: 1780 }
    ]
  },
  {
    id: "act_1_aegir",
    name: "Aegir",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1680, tradableGold: 2750, boundGold: 2750, boxPrice: 1820 },
      { gate: "Gate 2", minIlvl: 1680, tradableGold: 6250, boundGold: 6250, boxPrice: 4150 }
    ]
  },
  {
    id: "act_2_brelshaza",
    name: "Brelshaza",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1670, tradableGold: 2750, boundGold: 2750, boxPrice: 1820 },
      { gate: "Gate 2", minIlvl: 1670, tradableGold: 5500, boundGold: 5500, boxPrice: 3720 }
    ]
  },
  {
    id: "act_2_brelshaza",
    name: "Brelshaza",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1670, tradableGold: 2750, boundGold: 2750, boxPrice: 1820 },
      { gate: "Gate 2", minIlvl: 1670, tradableGold: 5500, boundGold: 5500, boxPrice: 3720 }
    ]
  },
  {
    id: "act_2_brelshaza",
    name: "Brelshaza",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1690, tradableGold: 3750, boundGold: 3750, boxPrice: 2400 },
      { gate: "Gate 2", minIlvl: 1690, tradableGold: 7750, boundGold: 7750, boxPrice: 5100 }
    ]
  },
  {
    id: "act_3_mordum",
    name: "Mordum",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1680, tradableGold: 2000, boundGold: 2000, boxPrice: 2400 },
      { gate: "Gate 2", minIlvl: 1680, tradableGold: 3500, boundGold: 3500, boxPrice: 3200 },
      { gate: "Gate 3", minIlvl: 1680, tradableGold: 5000, boundGold: 5000, boxPrice: 4200 }
    ]
  },
  {
    id: "act_3_mordum",
    name: "Mordum",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1680, tradableGold: 2000, boundGold: 2000, boxPrice: 2400 },
      { gate: "Gate 2", minIlvl: 1680, tradableGold: 3500, boundGold: 3500, boxPrice: 3200 },
      { gate: "Gate 3", minIlvl: 1680, tradableGold: 5000, boundGold: 5000, boxPrice: 4200 }
    ]
  },
  {
    id: "act_3_mordum",
    name: "Mordum",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1700, tradableGold: 2500, boundGold: 2500, boxPrice: 2700 },
      { gate: "Gate 2", minIlvl: 1700, tradableGold: 4000, boundGold: 4000, boxPrice: 4100 },
      { gate: "Gate 3", minIlvl: 1700, tradableGold: 7000, boundGold: 7000, boxPrice: 5800 }
    ]
  },
  {
    id: "overture_echidna",
    name: "Echidna",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1620, tradableGold: 0, boundGold: 1900, boxPrice: 310 },
      { gate: "Gate 2", minIlvl: 1620, tradableGold: 0, boundGold: 4200, boxPrice: 700 }
    ]
  },
  {
    id: "act_4_armoche",
    name: "Armoche",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1700, tradableGold: 6250, boundGold: 6250, boxPrice: 4000 },
      { gate: "Gate 2", minIlvl: 1700, tradableGold: 10250, boundGold: 10250, boxPrice: 6560 }
    ]
  },
  {
    id: "act_4_armoche",
    name: "Armoche",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1720, tradableGold: 15000, boundGold: 0, boxPrice: 4800 },
      { gate: "Gate 2", minIlvl: 1720, tradableGold: 27000, boundGold: 0, boxPrice: 8640 }
    ]
  },
  {
    id: "denouement_final_day",
    name: "Kazeros",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1710, tradableGold: 7000, boundGold: 7000, boxPrice: 4480 },
      { gate: "Gate 2", minIlvl: 1710, tradableGold: 13000, boundGold: 13000, boxPrice: 8320 }
    ]
  },
  {
    id: "denouement_final_day",
    name: "Kazeros",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1730, tradableGold: 17000, boundGold: 0, boxPrice: 5440 },
      { gate: "Gate 2", minIlvl: 1730, tradableGold: 35000, boundGold: 0, boxPrice: 11200 }
    ]
  },
  {
    id: "shadow_serca",
    name: "Serca",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1710, tradableGold: 7000, boundGold: 7000, boxPrice: 4480 },
      { gate: "Gate 2", minIlvl: 1710, tradableGold: 10500, boundGold: 10500, boxPrice: 6720 }
    ]
  },
  {
    id: "shadow_serca",
    name: "Serca",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1730, tradableGold: 17500, boundGold: 0, boxPrice: 5600 },
      { gate: "Gate 2", minIlvl: 1730, tradableGold: 26500, boundGold: 0, boxPrice: 8480 }
    ]
  },
  {
    id: "shadow_serca",
    name: "Serca",
    difficulty: "Nightmare",
    gates: [
      { gate: "Gate 1", minIlvl: 1740, tradableGold: 21000, boundGold: 0, boxPrice: 6720 },
      { gate: "Gate 2", minIlvl: 1740, tradableGold: 33000, boundGold: 0, boxPrice: 10560 }
    ]
  },
  {
    id: "horizon_cathedral",
    name: "Cathedral",
    difficulty: "Level 1",
    gates: [
      { gate: "Gate 1", minIlvl: 1700, tradableGold: 0, boundGold: 13500, boxPrice: 4320 },
      { gate: "Gate 2", minIlvl: 1700, tradableGold: 0, boundGold: 16500, boxPrice: 5280 }
    ]
  },
  {
    id: "horizon_cathedral",
    name: "Cathedral",
    difficulty: "Level 2",
    gates: [
      { gate: "Gate 1", minIlvl: 1720, tradableGold: 0, boundGold: 16000, boxPrice: 5120 },
      { gate: "Gate 2", minIlvl: 1720, tradableGold: 0, boundGold: 24000, boxPrice: 7680 }
    ]
  },
  {
    id: "horizon_cathedral",
    name: "Cathedral",
    difficulty: "Level 3",
    gates: [
      { gate: "Gate 1", minIlvl: 1750, tradableGold: 0, boundGold: 20000, boxPrice: 6400 },
      { gate: "Gate 2", minIlvl: 1750, tradableGold: 0, boundGold: 30000, boxPrice: 9600 }
    ]
  },
  {
    id: "overture_echidna",
    name: "Echidna",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1640, tradableGold: 1100, boundGold: 1100, boxPrice: 720 },
      { gate: "Gate 2", minIlvl: 1640, tradableGold: 2500, boundGold: 2500, boxPrice: 1630 }
    ]
  },
  {
    id: "behemoth",
    name: "Behemoth",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1620, tradableGold: 1100, boundGold: 1100, boxPrice: 1250 },
      { gate: "Gate 2", minIlvl: 1620, tradableGold: 2500, boundGold: 2500, boxPrice: 2000 }
    ]
  }
];
