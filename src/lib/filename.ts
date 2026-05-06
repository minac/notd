export function generateFilename(now: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, '0');
  return [
    now.getFullYear(),
    pad(now.getMonth() + 1),
    pad(now.getDate())
  ].join('-')
    + '-'
    + pad(now.getHours())
    + pad(now.getMinutes())
    + pad(now.getSeconds())
    + '.md';
}

export function resolveCollision(base: string, existing: ReadonlySet<string>): string {
  if (!existing.has(base)) return base;
  const stem = base.replace(/\.md$/, '');
  let i = 2;
  while (existing.has(`${stem}-${i}.md`)) i++;
  return `${stem}-${i}.md`;
}
