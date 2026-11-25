"use client";

import type {
  Answers,
  Education,
  Experience,
  LinkKind,
  Personal,
  ProfileLink,
  Project,
  Skill,
} from "@/api/client";
import { useState } from "react";
import { DEGREES, FIELDS_OF_STUDY } from "@/lib/degrees";
import {
  BulletList,
  CheckField,
  Field,
  ListEditor,
  DateField,
  SelectField,
  TextAreaField,
  TextField,
  YesNoField,
  formatMonth,
} from "./fields";

export const emptyEducation = (): Education => ({
  school: "",
  degree: "",
  field_of_study: "",
  minor: "",
  gpa: "",
  location: "",
  start: null,
  end: null,
  details: "",
});

export const emptyExperience = (): Experience => ({
  company: "",
  title: "",
  location: "",
  employment_type: "",
  start: null,
  end: null,
  bullets: [],
});

export const emptyProject = (): Project => ({
  name: "",
  summary: "",
  role: "",
  url: "",
  start: null,
  end: null,
  bullets: [],
  skills: [],
});

const grid = "grid gap-4 sm:grid-cols-2";

const PHONE_TYPES = [
  { value: "mobile", label: "Mobile" },
  { value: "home", label: "Home" },
  { value: "work", label: "Work" },
];

export function PersonalEditor({
  value: p,
  onChange,
}: {
  value: Personal;
  onChange: (v: Personal) => void;
}) {
  const set = (k: keyof Personal) => (v: string) => onChange({ ...p, [k]: v });
  return (
    <div className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-3">
        <TextField
          label="First name"
          name="first_name"
          required
          value={p.first_name}
          onChange={set("first_name")}
          autoComplete="given-name"
        />
        <TextField
          label="Middle name"
          name="middle_name"
          value={p.middle_name}
          onChange={set("middle_name")}
          autoComplete="additional-name"
        />
        <TextField
          label="Last name"
          name="last_name"
          required
          value={p.last_name}
          onChange={set("last_name")}
          autoComplete="family-name"
        />
      </div>
      <TextField
        label="Email"
        name="email"
        required
        type="email"
        value={p.email}
        onChange={set("email")}
        autoComplete="email"
      />
      <div className="grid gap-4 sm:grid-cols-[10rem_1fr]">
        <SelectField
          label="Phone type"
          name="phone_type"
          value={p.phone_type}
          onChange={set("phone_type")}
          options={PHONE_TYPES}
        />
        <TextField
          label="Phone"
          name="phone"
          type="tel"
          value={p.phone}
          onChange={set("phone")}
          autoComplete="tel"
        />
      </div>
      <TextField
        label="Street address"
        name="street"
        value={p.street}
        onChange={set("street")}
        autoComplete="street-address"
      />
      <div className="grid gap-4 sm:grid-cols-4">
        <TextField
          label="City"
          name="city"
          value={p.city}
          onChange={set("city")}
          className="sm:col-span-2"
        />
        <TextField label="State" name="region" value={p.region} onChange={set("region")} />
        <TextField
          label="ZIP code"
          name="postal_code"
          value={p.postal_code}
          onChange={set("postal_code")}
        />
      </div>
      <TextField label="Country" name="country" value={p.country} onChange={set("country")} />
      <TextField
        label="Headline"
        name="headline"
        value={p.headline}
        onChange={set("headline")}
        placeholder="e.g. QA engineer"
        hint="A one-line title for yourself."
      />
      <TextAreaField label="Summary" name="summary" value={p.summary} onChange={set("summary")} />
    </div>
  );
}

const LINK_KINDS: { value: LinkKind; label: string }[] = [
  { value: "linkedin", label: "LinkedIn" },
  { value: "github", label: "GitHub" },
  { value: "portfolio", label: "Portfolio" },
  { value: "other", label: "Other" },
];

export function LinksEditor({
  value,
  onChange,
}: {
  value: ProfileLink[];
  onChange: (v: ProfileLink[]) => void;
}) {
  return (
    <ListEditor
      items={value}
      onChange={onChange}
      newItem={(): ProfileLink => ({ kind: "linkedin", url: "" })}
      addLabel="Add link"
      itemLabel="Link"
      reorder
      render={(l, update) => (
        <div className="grid gap-4 sm:grid-cols-[10rem_1fr]">
          <Field label="Type">
            {(id) => (
              <select
                id={id}
                className="h-10 w-full rounded-xl bg-muted px-3 text-base md:text-sm"
                value={l.kind}
                onChange={(e) => update({ kind: e.target.value as LinkKind })}
              >
                {LINK_KINDS.map((k) => (
                  <option key={k.value} value={k.value}>
                    {k.label}
                  </option>
                ))}
              </select>
            )}
          </Field>
          <TextField
            label="Link"
            value={l.url}
            onChange={(url) => update({ url })}
            placeholder="linkedin.com/in/you"
          />
        </div>
      )}
    />
  );
}

export function EducationEditor({
  value,
  onChange,
}: {
  value: Education[];
  onChange: (v: Education[]) => void;
}) {
  return (
    <ListEditor
      items={value}
      onChange={onChange}
      newItem={emptyEducation}
      addLabel="Add education"
      itemLabel="Education"
      render={(e, update) => (
        <div className="space-y-4">
          <TextField
            label="School"
            name="school"
            required
            value={e.school}
            onChange={(school) => update({ school })}
          />
          <div className={grid}>
            <TextField
              label="Degree"
              name="degree"
              suggestions={DEGREES}
              value={e.degree}
              onChange={(degree) => update({ degree })}
              placeholder="Bachelor of Science"
            />
            <TextField
              label="Major"
              name="field_of_study"
              suggestions={FIELDS_OF_STUDY}
              value={e.field_of_study}
              onChange={(field_of_study) => update({ field_of_study })}
              placeholder="Computer Science"
            />
            <TextField
              label="Minor"
              name="minor"
              suggestions={FIELDS_OF_STUDY}
              value={e.minor}
              onChange={(minor) => update({ minor })}
            />
            <TextField
              label="GPA"
              name="gpa"
              value={e.gpa}
              onChange={(gpa) => update({ gpa })}
              placeholder="3.5/4.0"
            />
            <TextField
              label="Location"
              name="location"
              value={e.location}
              onChange={(location) => update({ location })}
              className="sm:col-span-2"
            />
            <DateField
              label="Start"
              name="start"
              value={e.start}
              onChange={(start) => update({ start })}
            />
            <DateField
              label="End (or expected)"
              name="end"
              value={e.end}
              onChange={(end) => update({ end })}
            />
          </div>
          <TextAreaField
            label="Details"
            name="details"
            value={e.details}
            onChange={(details) => update({ details })}
            hint="Honors, activities, relevant coursework."
          />
        </div>
      )}
    />
  );
}

const EMPLOYMENT_TYPES = [
  "Full-time",
  "Part-time",
  "Internship",
  "Contract",
  "Freelance",
] as const;

/** Month number for comparing dates (`YYYY-MM-DD`, by month); an open end is "now". */
function monthIndex(m: string | null, fallback: number) {
  if (!m) return fallback;
  const [y, mo] = m.split("-").map(Number);
  return y * 12 + mo - 1;
}

/**
 * Other entries whose dates overlap this one. Two jobs at once is normal,
 * so this is a note to double-check, not an error.
 */
function overlaps(items: Experience[], i: number) {
  const now = new Date();
  const today = now.getFullYear() * 12 + now.getMonth();
  const span = (e: Experience) =>
    e.start ? [monthIndex(e.start, today), monthIndex(e.end, today)] : null;
  const me = span(items[i]);
  if (!me) return [];
  return items.filter((other, j) => {
    const o = j === i ? null : span(other);
    return o !== null && o[0] < me[1] && me[0] < o[1];
  });
}

export function ExperienceEditor({
  value,
  onChange,
}: {
  value: Experience[];
  onChange: (v: Experience[]) => void;
}) {
  return (
    <ListEditor
      items={value}
      onChange={onChange}
      newItem={emptyExperience}
      addLabel="Add work experience"
      itemLabel="Work experience"
      render={(e, update, i) => (
        <ExperienceFields e={e} update={update} overlapsWith={overlaps(value, i)} />
      )}
    />
  );
}

/** "I currently work here" is its own choice: an empty end date alone isn't. */
function ExperienceFields({
  e,
  update,
  overlapsWith,
}: {
  e: Experience;
  update: (patch: Partial<Experience>) => void;
  overlapsWith: Experience[];
}) {
  const [current, setCurrent] = useState(() => !!e.start && !e.end);
  return (
    <div className="space-y-4">
      <div className={grid}>
        <TextField
          label="Job title"
          name="title"
          required
          value={e.title}
          onChange={(title) => update({ title })}
        />
        <TextField
          label="Company"
          name="company"
          required
          value={e.company}
          onChange={(company) => update({ company })}
        />
        <TextField
          label="Location"
          name="location"
          value={e.location}
          onChange={(location) => update({ location })}
        />
        <SelectField
          label="Job type"
          name="employment_type"
          value={e.employment_type}
          onChange={(employment_type) => update({ employment_type })}
          options={EMPLOYMENT_TYPES}
        />
        <DateField
          label="Start"
          name="start"
          value={e.start}
          onChange={(start) => update({ start })}
        />
        <DateField
          label="End"
          name="end"
          value={e.end}
          onChange={(end) => update({ end })}
          disabled={current}
        />
      </div>
      <CheckField
        label="I currently work here"
        checked={current}
        onChange={(on) => {
          setCurrent(on);
          if (on) update({ end: null });
        }}
      />
      {overlapsWith.length > 0 && (
        <p className="rounded-xl bg-warning/12 px-3 py-2 text-xs text-[color-mix(in_oklch,var(--warning),black_40%)] dark:text-warning">
          These dates overlap with{" "}
          {overlapsWith
            .map((o) => `${o.company || o.title || "another job"} (${formatMonth(o.start)} – ${o.end ? formatMonth(o.end) : "Present"})`)
            .join(" and ")}
          . That&apos;s fine if you held both; otherwise check the dates.
        </p>
      )}
      <BulletList value={e.bullets} onChange={(bullets) => update({ bullets })} />
    </div>
  );
}

export function ProjectEditor({
  value,
  onChange,
}: {
  value: Project[];
  onChange: (v: Project[]) => void;
}) {
  return (
    <ListEditor
      items={value}
      onChange={onChange}
      newItem={emptyProject}
      addLabel="Add project"
      itemLabel="Project"
      reorder
      render={(p, update) => (
        <div className="space-y-4">
          <div className={grid}>
            <TextField
              label="Name"
              name="name"
              required
              value={p.name}
              onChange={(name) => update({ name })}
            />
            <TextField
              label="Your role"
              name="role"
              value={p.role}
              onChange={(role) => update({ role })}
            />
            <TextField
              label="One-line summary"
              name="summary"
              value={p.summary}
              onChange={(summary) => update({ summary })}
              placeholder="Online banking platform"
              className="sm:col-span-2"
            />
            <TextField
              label="Link"
              name="url"
              value={p.url}
              onChange={(url) => update({ url })}
              placeholder="github.com/you/project"
              className="sm:col-span-2"
            />
            <DateField
              label="Start"
              name="start"
              value={p.start}
              onChange={(start) => update({ start })}
            />
            <DateField
              label="End"
              name="end"
              value={p.end}
              onChange={(end) => update({ end })}
            />
          </div>
          <TextField
            label="Technologies"
            name="skills"
            value={p.skills.join(", ")}
            onChange={(v) =>
              update({ skills: v.split(",").map((s) => s.trimStart()) })
            }
            hint="Comma-separated."
          />
          <BulletList value={p.bullets} onChange={(bullets) => update({ bullets })} />
        </div>
      )}
    />
  );
}

/**
 * Skills as text, one category per line: "Languages: Rust, Go". A line
 * without a label adds uncategorized skills.
 */
export function skillsToText(skills: Skill[]) {
  const groups = new Map<string, string[]>();
  for (const s of skills)
    groups.set(s.category, [...(groups.get(s.category) ?? []), s.name]);
  return [...groups]
    .map(([cat, names]) =>
      cat ? `${cat}: ${names.join(", ")}` : names.join(", "),
    )
    .join("\n");
}

export function textToSkills(text: string): Skill[] {
  return text.split("\n").flatMap((line) => {
    const m = line.match(/^([^:,]{1,40}):(.*)$/);
    const [category, items] = m ? [m[1].trim(), m[2]] : ["", line];
    return items
      .split(",")
      .map((name) => name.trim())
      .filter(Boolean)
      .map((name) => ({ name, category }));
  });
}

export function SkillsEditor({
  value,
  onChange,
}: {
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <TextAreaField
      label="Skills"
      value={value}
      onChange={onChange}
      placeholder={"Languages: Python, SQL\nTools: Git, Jira, Playwright"}
      hint='Comma-separated. Start a line with "Category:" to group skills.'
    />
  );
}

const GENDER = ["Male", "Female", "Non-binary", "Prefer not to say"] as const;
const RACE = [
  "American Indian or Alaska Native",
  "Asian",
  "Black or African American",
  "Native Hawaiian or Other Pacific Islander",
  "White",
  "Two or more races",
  "Prefer not to say",
] as const;
const YES_NO_DECLINE = ["Yes", "No", "Prefer not to say"] as const;
const VETERAN = [
  "I am a protected veteran",
  "I am not a protected veteran",
  "Prefer not to say",
] as const;
const DISABILITY = [
  "Yes, I have a disability",
  "No, I don't have a disability",
  "Prefer not to say",
] as const;
const ORIENTATION = [
  "Heterosexual",
  "Gay",
  "Lesbian",
  "Bisexual",
  "Queer",
  "Asexual",
  "Prefer not to say",
] as const;
const PRONOUNS = [
  "He/him",
  "She/her",
  "They/them",
  "Prefer not to say",
] as const;
const BASIS = [
  "U.S. citizen",
  "Green card",
  "H-1B",
  "F-1 OPT",
  "F-1 STEM OPT",
  "TN",
  "Other visa",
] as const;

export function AnswersEditor({
  value: a,
  onChange,
}: {
  value: Answers;
  onChange: (v: Answers) => void;
}) {
  const eeo = (k: keyof Answers["eeo"]) => (v: string) =>
    onChange({ ...a, eeo: { ...a.eeo, [k]: v } });
  return (
    <div className="space-y-6">
      <div className="space-y-4">
        <h3 className="text-sm font-semibold">Work authorization</h3>
        <TextField
          label="Countries where you want to work"
          value={a.work_countries.join(", ")}
          onChange={(v) =>
            onChange({
              ...a,
              work_countries: v.split(",").map((c) => c.trimStart()),
            })
          }
          hint="Comma-separated."
        />
        <div className={grid}>
          <YesNoField
            label="Are you a U.S. citizen?"
            value={a.us_citizen}
            onChange={(us_citizen) => onChange({ ...a, us_citizen })}
          />
          <YesNoField
            label="Are you legally authorized to work in the U.S.?"
            value={a.authorized_us}
            onChange={(authorized_us) => onChange({ ...a, authorized_us })}
          />
          <YesNoField
            label="Will you now or in the future need sponsorship?"
            value={a.needs_sponsorship}
            onChange={(needs_sponsorship) =>
              onChange({ ...a, needs_sponsorship })
            }
          />
          <SelectField
            label="Authorization basis"
            value={a.authorization_basis}
            onChange={(authorization_basis) =>
              onChange({ ...a, authorization_basis })
            }
            options={BASIS}
          />
        </div>
      </div>
      <div className="space-y-4">
        <div className="space-y-1">
          <h3 className="text-sm font-semibold">Equal employment</h3>
          <p className="text-xs text-muted-foreground">
            Optional. Stored encrypted and only used to fill these questions on
            applications.
          </p>
        </div>
        <div className={grid}>
          <SelectField
            label="Gender"
            value={a.eeo.gender}
            onChange={eeo("gender")}
            options={GENDER}
          />
          <SelectField
            label="Race"
            value={a.eeo.race}
            onChange={eeo("race")}
            options={RACE}
          />
          <SelectField
            label="Hispanic or Latino?"
            value={a.eeo.hispanic_latino}
            onChange={eeo("hispanic_latino")}
            options={YES_NO_DECLINE}
          />
          <SelectField
            label="Veteran status"
            value={a.eeo.veteran}
            onChange={eeo("veteran")}
            options={VETERAN}
          />
          <SelectField
            label="Disability"
            value={a.eeo.disability}
            onChange={eeo("disability")}
            options={DISABILITY}
          />
          <SelectField
            label="LGBTQ+?"
            value={a.eeo.lgbtq}
            onChange={eeo("lgbtq")}
            options={YES_NO_DECLINE}
          />
          <SelectField
            label="Sexual orientation"
            value={a.eeo.sexual_orientation}
            onChange={eeo("sexual_orientation")}
            options={ORIENTATION}
          />
          <SelectField
            label="Pronouns"
            value={a.eeo.pronouns}
            onChange={eeo("pronouns")}
            options={PRONOUNS}
          />
        </div>
      </div>
    </div>
  );
}
