export interface ListDiffResult {
  aOnly: string[];
  bOnly: string[];
  both: string[];
  all: string[];
}

/** Compares newline-delimited lists as sets, preserving first-seen order. */
export function diffLists(aText: string, bText: string): ListDiffResult {
  const a = uniqueLines(aText);
  const b = uniqueLines(bText);
  const aSet = new Set(a);
  const bSet = new Set(b);

  return {
    aOnly: a.filter((item) => !bSet.has(item)),
    bOnly: b.filter((item) => !aSet.has(item)),
    both: a.filter((item) => bSet.has(item)),
    all: [...a, ...b.filter((item) => !aSet.has(item))],
  };
}

function uniqueLines(text: string): string[] {
  const seen = new Set<string>();
  return text.split(/\r?\n/).filter((line) => {
    if (line === "" || seen.has(line)) return false;
    seen.add(line);
    return true;
  });
}
