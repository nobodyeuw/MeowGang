export type AppThemeId = 'outlaw' | 'plague-alchemist' | 'void-nebula' | 'sidereal-gold' | 'powi';

export type AppThemeOption = {
  id: AppThemeId;
  label: string;
  description: string;
};

// Theme metadata is kept separate from CSS tokens so Settings can render labels
// without knowing how each theme maps to CSS variables.
export const APP_THEME_OPTIONS: AppThemeOption[] = [
  {
    id: 'outlaw',
    label: 'Outlaw',
    description: 'Current dark orange LOA Tracker theme'
  },
  {
    id: 'plague-alchemist',
    label: 'Alchemist',
    description: 'Dark bio-chemical theme with radioactive mint accents'
  },
  {
    id: 'void-nebula',
    label: 'Void Space',
    description: 'Deep cosmic purple with cyan and nebula-pink accents'
  },
  {
    id: 'sidereal-gold',
    label: 'Sidereal Gold',
    description: 'Premium graphite surfaces with champagne gold accents'
  },
  {
    id: 'powi',
    label: 'Powi',
    description: 'Smoky graphite with soft rose quartz highlights'
  }
];

export const DEFAULT_APP_THEME: AppThemeId = 'outlaw';

export function isAppThemeId(value: string | null | undefined): value is AppThemeId {
  return APP_THEME_OPTIONS.some((theme) => theme.id === value);
}
