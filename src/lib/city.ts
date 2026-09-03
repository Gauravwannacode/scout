/**
 * City matching that survives India's dual place names.
 *
 * Sources are inconsistent: Devfolio writes "Bengaluru, Karnataka", Unstop
 * often writes "Bangalore", and a user will type whichever they grew up
 * saying. A plain substring test silently returns nothing for the other
 * spelling — a filter that looks like it works and quietly hides everything,
 * which is the worst kind of bug for a feature you only use occasionally.
 */
const ALIASES: string[][] = [
  ["bangalore", "bengaluru"],
  ["bombay", "mumbai"],
  ["calcutta", "kolkata"],
  ["madras", "chennai"],
  ["poona", "pune"],
  ["gurgaon", "gurugram"],
  ["mysore", "mysuru"],
  ["baroda", "vadodara"],
  ["trivandrum", "thiruvananthapuram"],
  ["cochin", "kochi"],
  ["pondicherry", "puducherry"],
  ["vizag", "visakhapatnam"],
  ["delhi", "new delhi"],
];

/** Every spelling of the given place, lowercased. Always includes the input. */
export function cityForms(city: string): string[] {
  const c = city.trim().toLowerCase();
  if (!c) return [];
  const group = ALIASES.find((g) => g.includes(c));
  return group ? [...new Set([c, ...group])] : [c];
}

/**
 * Whether an item is plausibly in or near the given city.
 *
 * Deliberately loose. Matching "Pune" against "Pune, Maharashtra, India" is
 * exactly what is wanted, and getting that right without a geocoding dataset
 * means comparing substrings rather than coordinates.
 */
export function matchesCity(
  location: string | null,
  isOnline: boolean | null,
  extraText: string,
  city: string,
): boolean {
  const forms = cityForms(city);
  if (forms.length === 0) return false;
  // An online event is not "near" anywhere, however its venue is written.
  if (isOnline === true) return false;

  const where = (location ?? "").toLowerCase();
  const haystack = `${where} ${extraText}`.toLowerCase();
  return forms.some((f) => where.includes(f) || haystack.includes(f));
}
