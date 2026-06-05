// Source of truth for trade-skill materials and fusion-material crafting assumptions.
// The planner uses this to compare "buy fusion material" vs "buy or self-gather raw materials and craft".

export type TradeSkillCategory = 'foraging' | 'logging' | 'mining' | 'hunting' | 'fishing' | 'excavation';

export interface TradeSkillMaterial {
  slug: string;
  name: string;
  category: TradeSkillCategory;
}

export interface StrongholdCraftingBonuses {
  craftingCostReduction: number;
  craftingGreatSuccessChance: number;
  specialCostReduction: number;
  specialGreatSuccessChance: number;
}

export interface FusionMaterialCraftDefinition {
  id: string;
  slug: string;
  name: string;
  tradeCategory: TradeSkillCategory;
  amountsVerified: boolean;
  baseCraftGoldCost: number;
  outputPerCraft: number;
  craftsPerBatch: number;
  maxBatches: number;
  recipeMaterials: Array<{ slug: string; amount: number }>;
}

export const DEFAULT_STRONGHOLD_CRAFTING_BONUSES: StrongholdCraftingBonuses = {
  craftingCostReduction: 0,
  craftingGreatSuccessChance: 0,
  specialCostReduction: 0,
  specialGreatSuccessChance: 0
};

export const TRADE_SKILL_MATERIALS: TradeSkillMaterial[] = [
  { slug: 'wild-flower', name: 'Wild Flower', category: 'foraging' },
  { slug: 'shy-wild-flower', name: 'Shy Wild Flower', category: 'foraging' },
  { slug: 'bright-wild-flower', name: 'Bright Wild Flower', category: 'foraging' },
  { slug: 'abidos-wild-flower', name: 'Abidos Wild Flower', category: 'foraging' },

  { slug: 'timber', name: 'Timber', category: 'logging' },
  { slug: 'tender-timber', name: 'Tender Timber', category: 'logging' },
  { slug: 'sturdy-timber', name: 'Sturdy Timber', category: 'logging' },
  { slug: 'abidos-timber', name: 'Abidos Timber', category: 'logging' },

  { slug: 'iron-ore', name: 'Iron Ore', category: 'mining' },
  { slug: 'heavy-iron-ore', name: 'Heavy Iron Ore', category: 'mining' },
  { slug: 'strong-iron-ore', name: 'Strong Iron Ore', category: 'mining' },
  { slug: 'abidos-iron-ore', name: 'Abidos Iron Ore', category: 'mining' },

  { slug: 'treated-meat', name: 'Treated Meat', category: 'hunting' },
  { slug: 'thick-raw-meat', name: 'Thick Raw Meat', category: 'hunting' },
  { slug: 'oreha-thick-meat', name: 'Oreha Thick Meat', category: 'hunting' },
  { slug: 'abidos-thick-raw-meat', name: 'Abidos Thick Raw Meat', category: 'hunting' },
  { slug: 'exotic-leather', name: 'Exotic Leather', category: 'hunting' },
  { slug: 'crystallized-hunting-bauble', name: 'Crystallized Hunting Bauble', category: 'hunting' },

  { slug: 'fish', name: 'Fish', category: 'fishing' },
  { slug: 'redflesh-fish', name: 'Redflesh Fish', category: 'fishing' },
  { slug: 'oreha-solar-carp', name: 'Oreha Solar Carp', category: 'fishing' },
  { slug: 'abidos-solar-carp', name: 'Abidos Solar Carp', category: 'fishing' },
  { slug: 'crystallized-fishing-bauble', name: 'Crystallized Fishing Bauble', category: 'fishing' },

  { slug: 'ancient-relic', name: 'Ancient Relic', category: 'excavation' },
  { slug: 'rare-relic', name: 'Rare Relic', category: 'excavation' },
  { slug: 'oreha-relic', name: 'Oreha Relic', category: 'excavation' },
  { slug: 'abidos-relic', name: 'Abidos Relic', category: 'excavation' },
  { slug: 'crystallized-excavating-bauble', name: 'Crystallized Excavation Bauble', category: 'excavation' },
  { slug: 'exotic-relic', name: 'Exotic Relic', category: 'excavation' }
];

const FUSION_OUTPUTS = {
  oreha: {
    slug: 'oreha-fusion-material',
    name: 'Oreha Fusion Material',
    lowAmount: 80,
    midAmount: 40,
    highAmount: 10,
    usesAbidosMaterial: false,
    baseCraftGoldCost: 205,
    amountsVerified: true
  },
  superiorOreha: {
    slug: 'superior-oreha-fusion-material',
    name: 'Superior Oreha Fusion Material',
    lowAmount: 128,
    midAmount: 64,
    highAmount: 16,
    usesAbidosMaterial: false,
    baseCraftGoldCost: 250,
    amountsVerified: true
  },
  primeOreha: {
    slug: 'prime-oreha-fusion-material',
    name: 'Prime Oreha Fusion Material',
    lowAmount: 142,
    midAmount: 69,
    highAmount: 52,
    usesAbidosMaterial: false,
    baseCraftGoldCost: 300,
    amountsVerified: true
  },
  abidos: {
    slug: 'abidos-fusion-material',
    name: 'Abidos Fusion Material',
    lowAmount: 86,
    midAmount: 45,
    highAmount: 33,
    usesAbidosMaterial: true,
    baseCraftGoldCost: 400,
    amountsVerified: true
  },
  superiorAbidos: {
    slug: 'superior-abidos-fusion-material',
    name: 'Superior Abidos Fusion Material',
    lowAmount: 86,
    midAmount: 45,
    highAmount: 33,
    usesAbidosMaterial: true,
    baseCraftGoldCost: 400,
    amountsVerified: false
  }
};

// Material order is low-rank, mid-rank, optional Oreha-rank, Abidos-rank.
// Only Fishing, Hunting, and Excavation have an Oreha-rank material for Oreha,
// Superior Oreha, and Prime Oreha fusion recipes. Abidos recipes can use all
// six trade-skill categories.
const FUSION_RECIPE_MATERIAL_ORDER: Record<TradeSkillCategory, [string, string, string, string]> = {
  foraging: ['wild-flower', 'shy-wild-flower', '', 'abidos-wild-flower'],
  logging: ['timber', 'tender-timber', '', 'abidos-timber'],
  mining: ['iron-ore', 'heavy-iron-ore', '', 'abidos-iron-ore'],
  hunting: ['thick-raw-meat', 'treated-meat', 'oreha-thick-meat', 'abidos-thick-raw-meat'],
  fishing: ['fish', 'redflesh-fish', 'oreha-solar-carp', 'abidos-solar-carp'],
  excavation: ['ancient-relic', 'rare-relic', 'oreha-relic', 'abidos-relic']
};

function createFusionRecipe(
  tradeCategory: TradeSkillCategory,
  output: (typeof FUSION_OUTPUTS)[keyof typeof FUSION_OUTPUTS]
): FusionMaterialCraftDefinition {
  const [lowSlug, midSlug, orehaSlug, abidosSlug] = FUSION_RECIPE_MATERIAL_ORDER[tradeCategory];
  const highSlug = output.usesAbidosMaterial ? abidosSlug : orehaSlug;

  return {
    id: `${tradeCategory}-${output.slug}`,
    slug: output.slug,
    name: output.name,
    tradeCategory,
    amountsVerified: output.amountsVerified ?? true,
    baseCraftGoldCost: output.baseCraftGoldCost,
    outputPerCraft: 10,
    craftsPerBatch: 10,
    maxBatches: 4,
    recipeMaterials: [
      { slug: lowSlug, amount: output.lowAmount },
      { slug: midSlug, amount: output.midAmount },
      { slug: highSlug, amount: output.highAmount }
    ]
  };
}

export const FUSION_MATERIAL_CRAFTS: FusionMaterialCraftDefinition[] = [
  ...(['fishing', 'hunting', 'excavation'] as TradeSkillCategory[]).flatMap((category) =>
    [FUSION_OUTPUTS.oreha, FUSION_OUTPUTS.superiorOreha, FUSION_OUTPUTS.primeOreha].map((output) =>
      createFusionRecipe(category, output)
    )
  ),
  ...Object.keys(FUSION_RECIPE_MATERIAL_ORDER).flatMap((category) =>
    [FUSION_OUTPUTS.abidos, FUSION_OUTPUTS.superiorAbidos].map((output) =>
      createFusionRecipe(category as TradeSkillCategory, output)
    )
  )
];

export function getTradeSkillCategoryForSlug(slug: string): TradeSkillCategory | undefined {
  return TRADE_SKILL_MATERIALS.find((material) => material.slug === slug)?.category;
}

export function getTradeSkillDisplayName(slug: string): string | undefined {
  return TRADE_SKILL_MATERIALS.find((material) => material.slug === slug)?.name;
}

export function getFusionCraftsByOutputSlug(slug: string): FusionMaterialCraftDefinition[] {
  return FUSION_MATERIAL_CRAFTS.filter((craft) => craft.slug === slug);
}

export function getFusionCraftsByTradeCategory(category: TradeSkillCategory): FusionMaterialCraftDefinition[] {
  return FUSION_MATERIAL_CRAFTS.filter((craft) => craft.tradeCategory === category);
}

export function getExpectedCraftOutput(outputPerCraft: number, greatSuccessChance: number): number {
  return outputPerCraft * (1 + Math.max(0, greatSuccessChance) / 100);
}

export function applyCraftCostReduction(baseCost: number, costReduction: number): number {
  return baseCost * (1 - Math.max(0, costReduction) / 100);
}
