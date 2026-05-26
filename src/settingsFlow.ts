export function parseAutoChangeMinutes(value: string): number {
  const minutes = Number(value);
  if (!Number.isFinite(minutes)) {
    return 0;
  }

  return Math.min(Math.max(Math.trunc(minutes), 0), 1440);
}
