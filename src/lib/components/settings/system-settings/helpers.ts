export function formatPath(path: string): string {
  if (!path) return 'No path selected';
  if (path.length > 60) {
    return `...${path.substring(path.length - 57)}`;
  }
  return path;
}

export function isPathValid(path: string): boolean {
  return path !== null && path !== undefined && path.trim().length > 0;
}
