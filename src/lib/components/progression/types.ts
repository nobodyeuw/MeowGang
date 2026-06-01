export interface EngravingRow {
  id: number;
  characterId: number;
  engravingName: string;
  booksRead: number;
  maxBooks: number;
  stoneBonus: number;
  isManualEntry: boolean;
  updatedAt: number;
}

export interface EquipmentEffect {
  label: string;
  value: string | number;
  grade?: string;
}

export interface EquipmentRow {
  id: number;
  characterId: number;
  slot: string;
  enhancementLevel: number | null;
  tier: string | null;
  quality: number | null;
  itemLevel: number | null;
  effectsJson: string | null;
  isManualEntry: boolean;
  updatedAt: number;
}

export interface GemRow {
  id: number;
  characterId: number;
  slotIndex: number;
  gemName: string;
  skillName: string;
  gemType: string;
  gemLevel: number;
  isBound: boolean;
  isManualEntry: boolean;
  updatedAt: number;
}

export interface GemTypeLabel {
  icon: string;
  label: string;
  bound: boolean;
}

export interface ProgressionSnapshot {
  characterId: number;
  engravings: EngravingRow[];
  equipment: EquipmentRow[];
  gems: GemRow[];
  goals: any[];
}
