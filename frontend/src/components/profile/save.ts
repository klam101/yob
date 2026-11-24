import {
  type Answers,
  type Education,
  type Experience,
  type Personal,
  type Profile,
  type ProfileLink,
  type Project,
  type ResumeDraft,
  type Skill,
  api,
} from "@/api/client";

/** Saving a section answers with the saved (cleaned-up) value or a message. */
type Saved<T> = { ok: true; value: T } | { ok: false; message: string };

async function done<T>(p: Promise<{ data?: T; error?: { message: string } }>): Promise<Saved<T>> {
  try {
    const { data, error } = await p;
    if (data !== undefined) return { ok: true, value: data };
    return { ok: false, message: error?.message ?? "Couldn't save. Try again." };
  } catch {
    return { ok: false, message: "Can't reach the API. Is the stack running?" };
  }
}

const noBlankBullets = <T extends { bullets: string[] }>(items: T[]) =>
  items.map((i) => ({ ...i, bullets: i.bullets.map((b) => b.trim()).filter(Boolean) }));

export const save = {
  personal: (body: Personal) => done(api.PUT("/profile/personal", { body })),
  links: (body: ProfileLink[]) => done(api.PUT("/profile/links", { body: body.filter((l) => l.url.trim()) })),
  educations: (body: Education[]) => done(api.PUT("/profile/educations", { body })),
  experiences: (body: Experience[]) => done(api.PUT("/profile/experiences", { body: noBlankBullets(body) })),
  projects: (body: Project[]) =>
    done(
      api.PUT("/profile/projects", {
        body: noBlankBullets(body).map((p) => ({ ...p, skills: p.skills.map((s) => s.trim()).filter(Boolean) })),
      }),
    ),
  skills: (body: Skill[]) => done(api.PUT("/profile/skills", { body })),
  answers: (body: Answers) =>
    done(
      api.PUT("/profile/answers", {
        body: { ...body, work_countries: body.work_countries.map((c) => c.trim()).filter(Boolean) },
      }),
    ),
};

export async function loadProfile(): Promise<Profile | null> {
  const { data } = await api.GET("/profile").catch(() => ({ data: undefined }));
  return data ?? null;
}

/** Uploads a PDF or DOCX and returns the parsed draft (nothing is saved). */
export async function uploadResume(file: File): Promise<Saved<ResumeDraft>> {
  if (file.size > 5 * 1024 * 1024) return { ok: false, message: "The file is larger than 5 MB." };
  const form = new FormData();
  form.append("file", file);
  try {
    const res = await fetch("/api/profile/resume", { method: "POST", body: form });
    const body = await res.json().catch(() => null);
    if (res.ok && body) return { ok: true, value: body as ResumeDraft };
    if (res.status === 413) return { ok: false, message: "The file is larger than 5 MB." };
    return { ok: false, message: body?.message ?? "Couldn't read that file." };
  } catch {
    return { ok: false, message: "Can't reach the API. Is the stack running?" };
  }
}

export function isEmpty(p: Profile) {
  return (
    !p.personal.first_name &&
    !p.personal.last_name &&
    !p.experiences.length &&
    !p.educations.length &&
    !p.skills.length &&
    !p.projects.length
  );
}
