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
  difficulty: "Solo" | "Normal" | "Hard" | "Nightmare";
  gates: RaidGate[];
}

export const RAIDS: Raid[] = [
  {
    id: "overture_echidna",
    name: "Echidna",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1620, tradableGold: 0, boundGold: 3500, boxPrice: 310 },
      { gate: "Gate 2", minIlvl: 1620, tradableGold: 0, boundGold: 5600, boxPrice: 700 }
    ]
  },
  {
    id: "act_1_aegir",
    name: "Aegir",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1660, tradableGold: 0, boundGold: 7000, boxPrice: 750 },
      { gate: "Gate 2", minIlvl: 1660, tradableGold: 0, boundGold: 9800, boxPrice: 1780 }
    ]
  },
  {
    id: "act_1_aegir",
    name: "Aegir",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1660, tradableGold: 3500, boundGold: 3500, boxPrice: 750 },
      { gate: "Gate 2", minIlvl: 1660, tradableGold: 4900, boundGold: 4900, boxPrice: 1780 }
    ]
  },
  {
    id: "act_1_aegir",
    name: "Aegir",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1680, tradableGold: 3500, boundGold: 3500, boxPrice: 1820 },
      { gate: "Gate 2", minIlvl: 1680, tradableGold: 7000, boundGold: 7000, boxPrice: 4150 }
    ]
  },
  {
    id: "act_2_brelshaza",
    name: "Brelshaza",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1670, tradableGold: 0, boundGold: 6300, boxPrice: 1820 },
      { gate: "Gate 2", minIlvl: 1670, tradableGold: 0, boundGold: 12950, boxPrice: 3720 }
    ]
  },
  {
    id: "act_2_brelshaza",
    name: "Brelshaza",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1670, tradableGold: 3150, boundGold: 3150, boxPrice: 1820 },
      { gate: "Gate 2", minIlvl: 1670, tradableGold: 6475, boundGold: 6475, boxPrice: 3720 }
    ]
  },
  {
    id: "act_2_brelshaza",
    name: "Brelshaza",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1690, tradableGold: 3850, boundGold: 3850, boxPrice: 2400 },
      { gate: "Gate 2", minIlvl: 1690, tradableGold: 8050, boundGold: 8050, boxPrice: 5100 }
    ]
  },
  {
    id: "act_3_mordum",
    name: "Mordum",
    difficulty: "Solo",
    gates: [
      { gate: "Gate 1", minIlvl: 1680, tradableGold: 0, boundGold: 6000, boxPrice: 2400 },
      { gate: "Gate 2", minIlvl: 1680, tradableGold: 0, boundGold: 9500, boxPrice: 3200 },
      { gate: "Gate 3", minIlvl: 1680, tradableGold: 0, boundGold: 12500, boxPrice: 4200 }
    ]
  },
  {
    id: "act_3_mordum",
    name: "Mordum",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1680, tradableGold: 4200, boundGold: 1800, boxPrice: 2400 },
      { gate: "Gate 2", minIlvl: 1680, tradableGold: 6650, boundGold: 2850, boxPrice: 3200 },
      { gate: "Gate 3", minIlvl: 1680, tradableGold: 8750, boundGold: 3750, boxPrice: 4200 }
    ]
  },
  {
    id: "act_3_mordum",
    name: "Mordum",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1700, tradableGold: 4900, boundGold: 2100, boxPrice: 2700 },
      { gate: "Gate 2", minIlvl: 1700, tradableGold: 7700, boundGold: 3300, boxPrice: 4100 },
      { gate: "Gate 3", minIlvl: 1700, tradableGold: 14000, boundGold: 6000, boxPrice: 5800 }
    ]
  },
  {
    id: "overture_echidna",
    name: "Echidna",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1620, tradableGold: 1750, boundGold: 1750, boxPrice: 310 },
      { gate: "Gate 2", minIlvl: 1620, tradableGold: 2800, boundGold: 2800, boxPrice: 700 }
    ]
  },
  {
    id: "act_4_armoche",
    name: "Armoche",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1700, tradableGold: 12500, boundGold: 0, boxPrice: 4000 },
      { gate: "Gate 2", minIlvl: 1700, tradableGold: 20500, boundGold: 0, boxPrice: 6560 }
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
      { gate: "Gate 1", minIlvl: 1710, tradableGold: 14000, boundGold: 0, boxPrice: 4480 },
      { gate: "Gate 2", minIlvl: 1710, tradableGold: 26000, boundGold: 0, boxPrice: 8320 }
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
      { gate: "Gate 1", minIlvl: 1710, tradableGold: 14000, boundGold: 0, boxPrice: 4480 },
      { gate: "Gate 2", minIlvl: 1710, tradableGold: 21000, boundGold: 0, boxPrice: 6720 }
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
    id: "overture_echidna",
    name: "Echidna",
    difficulty: "Hard",
    gates: [
      { gate: "Gate 1", minIlvl: 1630, tradableGold: 2100, boundGold: 2800, boxPrice: 720 },
      { gate: "Gate 2", minIlvl: 1630, tradableGold: 2800, boundGold: 3500, boxPrice: 1630 }
    ]
  },
  {
    id: "behemoth",
    name: "Behemoth",
    difficulty: "Normal",
    gates: [
      { gate: "Gate 1", minIlvl: 1620, tradableGold: 1750, boundGold: 1750, boxPrice: 1250 },
      { gate: "Gate 2", minIlvl: 1620, tradableGold: 2800, boundGold: 2800, boxPrice: 2000 }
    ]
  }
];
