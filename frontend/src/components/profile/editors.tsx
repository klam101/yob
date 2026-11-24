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
import {
  Field,
  BulletsField,
  CheckField,
  ListEditor,
  MonthField,
  SelectField,
  TextAreaField,
  TextField,
  YesNoField,
} from "./fields";

export const emptyEducation = (): Education => ({
  school: "",
  degree: "",
  field_of_study: "",
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
  role: "",
  url: "",
  start: null,
  end: null,
  bullets: [],
  skills: [],
});

const grid = "grid gap-4 sm:grid-cols-2";

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
      <div className={grid}>
        <TextField
          label="First name"
          value={p.first_name}
          onChange={set("first_name")}
          autoComplete="given-name"
        />
        <TextField
          label="Last name"
          value={p.last_name}
          onChange={set("last_name")}
          autoComplete="family-name"
        />
        <TextField
          label="Email"
          type="email"
          value={p.email}
          onChange={set("email")}
          autoComplete="email"
        />
        <TextField
          label="Phone"
          type="tel"
          value={p.phone}
          onChange={set("phone")}
          autoComplete="tel"
        />
      </div>
      <TextField
        label="Street address"
        value={p.street}
        onChange={set("street")}
        autoComplete="street-address"
      />
      <div className="grid gap-4 sm:grid-cols-4">
        <TextField
          label="City"
          value={p.city}
          onChange={set("city")}
          className="sm:col-span-2"
        />
        <TextField label="State" value={p.region} onChange={set("region")} />
        <TextField
          label="ZIP code"
          value={p.postal_code}
          onChange={set("postal_code")}
        />
      </div>
      <TextField label="Country" value={p.country} onChange={set("country")} />
      <TextField
        label="Headline"
        value={p.headline}
        onChange={set("headline")}
        placeholder="e.g. QA engineer"
        hint="A one-line title for yourself."
      />
      <TextAreaField
        label="Summary"
        value={p.summary}
        onChange={set("summary")}
      />
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
      itemLabel={(l) =>
        LINK_KINDS.find((k) => k.value === l.kind)?.label ?? "Link"
      }
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
      itemLabel={(e, i) => e.school || `Education ${i + 1}`}
      render={(e, update) => (
        <div className="space-y-4">
          <TextField
            label="School"
            value={e.school}
            onChange={(school) => update({ school })}
          />
          <div className={grid}>
            <TextField
              label="Degree"
              value={e.degree}
              onChange={(degree) => update({ degree })}
              placeholder="Bachelor of Science"
            />
            <TextField
              label="Field of study"
              value={e.field_of_study}
              onChange={(field_of_study) => update({ field_of_study })}
            />
            <TextField
              label="GPA"
              value={e.gpa}
              onChange={(gpa) => update({ gpa })}
              placeholder="3.5/4.0"
            />
            <TextField
              label="Location"
              value={e.location}
              onChange={(location) => update({ location })}
            />
            <MonthField
              label="Start"
              value={e.start}
              onChange={(start) => update({ start })}
            />
            <MonthField
              label="End (or expected)"
              value={e.end}
              onChange={(end) => update({ end })}
            />
          </div>
          <TextAreaField
            label="Details"
            value={e.details}
            onChange={(details) => update({ details })}
            hint="Honors, minor, relevant coursework."
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
      addLabel="Add experience"
      itemLabel={(e, i) =>
        [e.title, e.company].filter(Boolean).join(" · ") ||
        `Experience ${i + 1}`
      }
      render={(e, update) => <ExperienceFields e={e} update={update} />}
    />
  );
}

/** "I currently work here" is its own choice: an empty end date alone isn't. */
function ExperienceFields({
  e,
  update,
}: {
  e: Experience;
  update: (patch: Partial<Experience>) => void;
}) {
  const [current, setCurrent] = useState(() => !!e.start && !e.end);
  return (
    <div className="space-y-4">
      <div className={grid}>
        <TextField
          label="Title"
          value={e.title}
          onChange={(title) => update({ title })}
        />
        <TextField
          label="Company"
          value={e.company}
          onChange={(company) => update({ company })}
        />
        <TextField
          label="Location"
          value={e.location}
          onChange={(location) => update({ location })}
        />
        <SelectField
          label="Employment type"
          value={e.employment_type}
          onChange={(employment_type) => update({ employment_type })}
          options={EMPLOYMENT_TYPES}
        />
        <MonthField
          label="Start"
          value={e.start}
          onChange={(start) => update({ start })}
        />
        <MonthField
          label="End"
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
      <BulletsField
        value={e.bullets}
        onChange={(bullets) => update({ bullets })}
      />
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
      itemLabel={(p, i) => p.name || `Project ${i + 1}`}
      render={(p, update) => (
        <div className="space-y-4">
          <div className={grid}>
            <TextField
              label="Name"
              value={p.name}
              onChange={(name) => update({ name })}
            />
            <TextField
              label="Your role"
              value={p.role}
              onChange={(role) => update({ role })}
            />
            <TextField
              label="Link"
              value={p.url}
              onChange={(url) => update({ url })}
              className="sm:col-span-2"
            />
            <MonthField
              label="Start"
              value={p.start}
              onChange={(start) => update({ start })}
            />
            <MonthField
              label="End"
              value={p.end}
              onChange={(end) => update({ end })}
            />
          </div>
          <TextField
            label="Technologies"
            value={p.skills.join(", ")}
            onChange={(v) =>
              update({ skills: v.split(",").map((s) => s.trimStart()) })
            }
            hint="Comma-separated."
          />
          <BulletsField
            value={p.bullets}
            onChange={(bullets) => update({ bullets })}
          />
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
