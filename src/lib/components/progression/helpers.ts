import type {
  EngravingRow,
  EquipmentEffect,
  EquipmentRow,
  GemRow,
  GemTypeLabel
} from '$lib/components/progression/types';

export const SLOT_LABELS: Record<string, string> = {
  weapon: 'Weapon',
  head: 'Head',
  chest: 'Chest',
  pants: 'Pants',
  gloves: 'Gloves',
  shoulder: 'Shoulder',
  neck: 'Necklace',
  earring1: 'Earring 1',
  earring2: 'Earring 2',
  ring1: 'Ring 1',
  ring2: 'Ring 2',
  bracelet: 'Bracelet',
  ability_stone: 'Ability Stone'
};

export const ARMOR_ORDER = ['head', 'shoulder', 'chest', 'pants', 'gloves', 'weapon'];
export const ACCESSORY_ORDER = ['neck', 'earring1', 'earring2', 'ring1', 'ring2', 'ability_stone'];

export function qualityColor(quality: number | null): string {
  if (quality === null) return 'var(--md-sys-color-outline-variant)';
  if (quality >= 100) return '#f59e0b';
  if (quality >= 90) return '#a855f7';
  if (quality >= 70) return '#3b82f6';
  if (quality >= 30) return '#10b981';
  return 'var(--md-sys-color-on-surface-variant)';
}

export function gemTypeLabel(gemType: string, isBound: boolean): GemTypeLabel {
  if (gemType === 'attack') return { icon: 'ATK', label: 'Atk', bound: isBound };
  if (gemType === 'cooldown') return { icon: 'CD', label: 'CD', bound: isBound };
  return { icon: 'Gem', label: gemType, bound: isBound };
}

export function gemLevelColor(level: number): string {
  if (level >= 10) return '#f59e0b';
  if (level >= 8) return '#a855f7';
  if (level >= 6) return '#3b82f6';
  return 'var(--md-sys-color-on-surface-variant)';
}

export function engravingNodes(booksRead: number, stoneBonus: number): number {
  return Math.floor(booksRead / 5) + stoneBonus;
}

export function lastScrapedLabel(updatedAt: number): string {
  if (!updatedAt) return '';
  const date = new Date(updatedAt * 1000);
  return date.toLocaleString();
}

export function equipmentEffects(item: EquipmentRow | null): EquipmentEffect[] {
  if (!item?.effectsJson) return [];

  try {
    const parsed = JSON.parse(item.effectsJson);
    return Array.isArray(parsed)
      ? parsed.filter((effect): effect is EquipmentEffect =>
          Boolean(effect?.label) && effect?.value !== undefined && effect?.value !== null
        )
      : [];
  } catch {
    return [];
  }
}

export function visibleEquipmentEffects(item: EquipmentRow | null): EquipmentEffect[] {
  return equipmentEffects(item).filter((effect) => effect.label !== 'Vitality' && effect.label !== 'Base Stat');
}

export function hoverEquipmentEffects(item: EquipmentRow | null): EquipmentEffect[] {
  return equipmentEffects(item).filter((effect) => effect.label === 'Vitality' || effect.label === 'Base Stat');
}

export function formatEffectValue(value: string | number): string {
  if (typeof value === 'number') {
    return value > 0 ? `+${value}` : `${value}`;
  }
  return value;
}

export function sortedEngravings(engravings: EngravingRow[]): EngravingRow[] {
  return [...engravings].sort((a, b) => b.booksRead + b.stoneBonus - (a.booksRead + a.stoneBonus));
}

export function sortedGems(gems: GemRow[]): GemRow[] {
  return [...gems].sort((a, b) => a.slotIndex - b.slotIndex);
}

export function gemSocketRows(gems: GemRow[]): GemRow[][] {
  const sorted = sortedGems(gems);
  return [sorted.slice(0, 4), sorted.slice(4, 7), sorted.slice(7, 11)].filter((row) => row.length > 0);
}
