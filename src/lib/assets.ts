// Central asset resolver for images moved from static/ into Vite-managed src/assets/.
// Dynamic callers pass folder/name pairs; Vite still fingerprints and bundles the matched files.
const assetModules = import.meta.glob('/src/assets/**/*', {
  eager: true,
  query: '?url',
  import: 'default'
}) as Record<string, string>;

export function assetUrl(path: string): string {
  const normalizedPath = path.replace(/^\/+/, '');
  const resolvedAsset = assetModules[`/src/assets/${normalizedPath}`];

  if (!resolvedAsset) {
    console.warn(`Missing bundled asset: src/assets/${normalizedPath}`);
    return '';
  }

  return resolvedAsset;
}

export function appAsset(name: string): string {
  return assetUrl(`app/${name}`);
}

export function classAsset(iconId: string | number): string {
  return assetUrl(`classes/${iconId}.png`);
}

export function iconAsset(name: string): string {
  return assetUrl(`icons/${name}`);
}

export function marketAsset(name: string): string {
  return assetUrl(`market/${name}`);
}
