/**
 * "QA ENGINEER" -> "QA Engineer". A port of `tidy_caps` in
 * backend/crates/core/src/profile/parse.rs; keep the word lists and rules
 * the same in both places.
 */

const SMALL_WORDS = new Set(["of", "in", "and", "the", "for", "at", "to", "a", "an", "with", "on", "or", "by", "&", "via"]);

// Short everyday words, title-cased even though most short words in an
// all-caps title are acronyms ("MAP" -> "Map").
const SHORT_WORDS = new Set([
  "app", "map", "web", "art", "new", "big", "bot", "car", "day", "job", "key", "lab", "law",
  "pay", "sea", "sun", "top", "use", "way", "toy", "tax", "tea", "pet", "box", "hub", "kit",
  "net", "pro", "tag", "cup", "gym", "bar", "bus", "fun", "end", "all", "one", "two", "six",
  "ten", "our", "old", "red", "my", "go", "run", "air", "low", "sky", "bay", "aid", "fit",
]);

const ACRONYMS = new Set([
  "GPA", "QA", "QE", "AI", "ML", "UI", "UX", "IT", "MVP", "AWS", "GCP", "SQL", "API", "LLC",
  "LLP", "CEO", "CTO", "CFO", "COO", "VP", "HR", "USA", "US", "U.S.", "NASA", "IBM", "MBA",
  "PHD", "B.S.", "B.A.", "M.S.", "M.A.", "BS", "BA", "MS", "MA", "II", "III", "IV", "R&D",
  "SDET", "SRE", "DBA", "ERP", "CRM", "GIS", "IOS", "SAAS", "POS", "STEM", "NLP", "CS", "CE",
  "EE", "ECE", "DEVOPS", "CI/CD", "ETL", "BI", "PM", "TA", "RA",
]);

const SPECIAL: Record<string, string> = { IOS: "iOS", SAAS: "SaaS", PHD: "PhD", DEVOPS: "DevOps" };

const letters = (s: string) => [...s].filter((c) => c.toLowerCase() !== c.toUpperCase());
const isUpper = (c: string) => c !== c.toLowerCase() && c === c.toUpperCase();

/** "JANE" -> "Jane"; mixed case is kept. */
function titleCase(w: string) {
  const l = letters(w);
  if ([...w].length > 1 && l.length > 0 && l.every(isUpper)) {
    return w.charAt(0).toUpperCase() + w.slice(1).toLowerCase();
  }
  return w;
}

/** Text written in capitals: more than four letters, none lowercase. */
export function isAllCaps(s: string) {
  const l = letters(s);
  return l.length > 4 && l.every(isUpper);
}

/** Title-cases all-caps text; anything with lowercase letters is kept. */
export function tidyCaps(s: string) {
  if (!isAllCaps(s)) return s;
  return s
    .split(" ")
    .map((w, i) => {
      const bare = w.toUpperCase().replace(/^[,;:()]+|[,;:()]+$/g, "");
      if (ACRONYMS.has(bare)) return SPECIAL[bare] ? w.replace(bare, SPECIAL[bare]) : w;
      if (i > 0 && SMALL_WORDS.has(w.toLowerCase())) return w.toLowerCase();
      if (letters(w).length <= 3 && !SHORT_WORDS.has(w.toLowerCase())) return w;
      return w.split("-").map(titleCase).join("-");
    })
    .join(" ");
}
