export function fuzzyScore(text: string, query: string): number {
  if (!query) return 0;

  const t = text.toLowerCase();
  const q = query.toLowerCase();

  if (t === q) return 1000;
  if (t.startsWith(q)) return 500 + (100 - q.length);

  const idx = t.indexOf(q);
  if (idx !== -1) return 100 - idx;

  let ti = 0;
  let qi = 0;
  while (ti < t.length && qi < q.length) {
    if (t[ti] === q[qi]) qi++;
    ti++;
  }
  if (qi === q.length) return Math.max(1, 50 - ti);

  return -1;
}

export interface FuzzyResult<T> {
  item: T;
  score: number;
}

export function fuzzyFilter<T>(
  items: T[],
  getText: (item: T) => string,
  query: string,
): FuzzyResult<T>[] {
  const results: FuzzyResult<T>[] = [];
  for (const item of items) {
    const score = fuzzyScore(getText(item), query);
    if (score >= 0) results.push({ item, score });
  }
  return results.sort((a, b) => b.score - a.score);
}
