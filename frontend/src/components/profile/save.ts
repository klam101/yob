import {
  type Answers,
  type Education,
  type Experience,
  type Personal,
  type Profile,
  type ProfileLink,
  type Project,
  type ResumeContent,
  type ProfileSection,
  type ResumeUploaded,
  type ResumeUpdate,
  type Skill,
  api,
} from "@/api/client";

/**
 * Saving answers with the saved (cleaned-up) value, or a message and, when
 * it's about one field, that field's path (`experiences.2.end`).
 */
export type Saved<T> = { ok: true; value: T } | { ok: false; message: string; field?: string | null };

async function done<T>(
  p: Promise<{ data?: T; error?: { message: string; field?: string | null } }>,
): Promise<Saved<T>> {
  try {
    const { data, error } = await p;
    if (data !== undefined) return { ok: true, value: data };
    return { ok: false, message: error?.message ?? "Couldn't save. Try again.", field: error?.field };
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

export const resumes = {
  list: () => done(api.GET("/profile/resumes")),
  update: (id: string, body: ResumeUpdate) =>
    done(api.PATCH("/profile/resumes/{id}", { params: { path: { id } }, body })),
  remove: async (id: string): Promise<Saved<null>> => {
    try {
      const { response, error } = await api.DELETE("/profile/resumes/{id}", { params: { path: { id } } });
      if (response.ok) return { ok: true, value: null };
      return { ok: false, message: error?.message ?? "Couldn't delete it. Try again." };
    } catch {
      return { ok: false, message: "Can't reach the API. Is the stack running?" };
    }
  },
  detail: (id: string) => done(api.GET("/profile/resumes/{id}", { params: { path: { id } } })),
  apply: (id: string, sections: ProfileSection[]) =>
    done(api.POST("/profile/resumes/{id}/apply", { params: { path: { id } }, body: { sections } })),
  saveContent: (id: string, body: ResumeContent) =>
    done(api.PUT("/profile/resumes/{id}/content", { params: { path: { id } }, body })),
  reparse: (id: string) => done(api.POST("/profile/resumes/{id}/reparse", { params: { path: { id } } })),
};

/** Hides the "filled from your resume" banner. */
export async function fillReviewed() {
  await api.POST("/profile/fill-reviewed").catch(() => undefined);
}

export async function loadProfile(): Promise<Profile | null> {
  const { data } = await api.GET("/profile").catch(() => ({ data: undefined }));
  return data ?? null;
}

/** Uploads a PDF or DOCX and returns the parsed draft (nothing is saved). */
export async function uploadResume(file: File): Promise<Saved<ResumeUploaded>> {
  if (file.size > 5 * 1024 * 1024) return { ok: false, message: "The file is larger than 5 MB." };
  const form = new FormData();
  form.append("file", file);
  try {
    const res = await fetch("/api/profile/resume", { method: "POST", body: form });
    const body = await res.json().catch(() => null);
    if (res.ok && body) return { ok: true, value: body as ResumeUploaded };
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
