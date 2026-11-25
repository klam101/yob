/**
 * Suggestions for degrees and fields of study. Only suggestions: any text is
 * allowed, so "B.S." from a resume stays as written. The menus show the first
 * few when nothing is typed, so each list starts with the common choices.
 */

/** Degrees, for the profile's one-degree-per-entry Degree field. */
export const DEGREES = [
  "Bachelor's",
  "Bachelor of Science (BS)",
  "Bachelor of Arts (BA)",
  "Master's",
  "Master of Science (MS)",
  "Associate's",
  "Doctor of Philosophy (PhD)",
  "Master of Business Administration (MBA)",
  "Bachelor's Degree",
  "Master's Degree",
  "Associate's Degree",
  "High school diploma",
  "GED",
  "Certificate",
  "Associate of Arts (AA)",
  "Associate of Science (AS)",
  "Associate of Applied Science (AAS)",
  "Bachelor of Fine Arts (BFA)",
  "Bachelor of Engineering (BEng)",
  "Bachelor of Business Administration (BBA)",
  "Master of Arts (MA)",
  "Master of Engineering (MEng)",
  "Master of Fine Arts (MFA)",
  "Master of Public Health (MPH)",
  "Master of Social Work (MSW)",
  "Doctorate",
  "Doctor of Medicine (MD)",
  "Juris Doctor (JD)",
  "Doctor of Education (EdD)",
] as const;

/**
 * The first box of a resume's "X in Y" rows: a degree, or "Major", "Minor"
 * or "Concentration" for what goes with the degree above it.
 */
export const DEGREE_ROWS = [
  "Bachelor's",
  "Bachelor of Science (BS)",
  "Bachelor of Arts (BA)",
  "Major",
  "Minor",
  "Master's",
  "Master of Science (MS)",
  "Concentration",
  ...DEGREES.slice(5),
  "Specialization",
] as const;

export const FIELDS_OF_STUDY = [
  "Computer Science",
  "Business Administration",
  "Psychology",
  "Nursing",
  "Biology",
  "Mechanical Engineering",
  "Electrical Engineering",
  "Economics",
  "Accounting",
  "Aerospace Engineering",
  "Anthropology",
  "Applied Mathematics",
  "Architecture",
  "Art",
  "Biochemistry",
  "Biomedical Engineering",
  "Chemical Engineering",
  "Chemistry",
  "Civil Engineering",
  "Communications",
  "Computer Engineering",
  "Computer Information Systems",
  "Criminal Justice",
  "Cybersecurity",
  "Data Science",
  "Design",
  "Education",
  "English",
  "Environmental Science",
  "Finance",
  "Graphic Design",
  "Health Sciences",
  "History",
  "Industrial Engineering",
  "Information Systems",
  "Information Technology",
  "International Relations",
  "Journalism",
  "Kinesiology",
  "Linguistics",
  "Management",
  "Management Information Systems",
  "Marketing",
  "Materials Science",
  "Mathematics",
  "Music",
  "Philosophy",
  "Physics",
  "Political Science",
  "Public Health",
  "Public Policy",
  "Sociology",
  "Software Engineering",
  "Statistics",
  "Supply Chain Management",
] as const;

/**
 * Whether a row's first box names a degree, or something that goes with the
 * degree above it. Mirrors `row_kind` in the API's `resume_content.rs`, which
 * turns rows into the profile's one-degree entries.
 */
export function rowKind(first: string): "degree" | "major" | "minor" | "concentration" {
  const d = first.trim().toLowerCase();
  const starts = (words: string[]) => words.some((w) => d.startsWith(w));
  if (starts(["minor"])) return "minor";
  if (starts(["major", "double major", "second major", "co-major"])) return "major";
  if (starts(["concentration", "specialization", "specialisation", "emphasis", "track", "option in"])) return "concentration";
  return "degree";
}
