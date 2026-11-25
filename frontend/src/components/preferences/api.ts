import { type Preferences, type SearchPreferences, api } from "@/api/client";

type Result = { ok: true; value: SearchPreferences } | { ok: false; message: string };

async function done(p: Promise<{ data?: SearchPreferences; error?: { message: string } }>): Promise<Result> {
  try {
    const { data, error } = await p;
    if (data) return { ok: true, value: data };
    return { ok: false, message: error?.message ?? "Couldn't save. Try again." };
  } catch {
    return { ok: false, message: "Can't reach the API. Is the stack running?" };
  }
}

/** Saves every answer and marks the questionnaire done. */
export const savePreferences = (body: Preferences) => done(api.PUT("/preferences", { body }));

/** "Skip for now": the feed stops sending you to the questionnaire. */
export const skipPreferences = () => done(api.POST("/preferences/skip"));
