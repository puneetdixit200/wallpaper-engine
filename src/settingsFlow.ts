export function parseAutoChangeMinutes(value: string): number {
  const minutes = Number(value);
  if (!Number.isFinite(minutes)) {
    return 0;
  }

  return Math.min(Math.max(Math.trunc(minutes), 0), 1440);
}

export function parseCacheLimitMb(value: string): number {
  const megabytes = Number(value);
  if (!Number.isFinite(megabytes)) {
    return 1024;
  }

  return Math.min(Math.max(Math.trunc(megabytes), 128), 10240);
}
