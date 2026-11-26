# yob brand guide

yob is a job search platform: one central list of jobs with easy apply.
Brand personality: trustworthy, dependable, gets the job done.

## Where things live

- Logos, wordmarks, the icon and the gold mark: `frontend/public/brand/` (served at `/brand/...`).
- Favicon and Apple touch icon: `frontend/src/app/icon.svg` and `apple-icon.png` (Next.js file conventions); the web app manifest is `frontend/src/app/manifest.ts`.
- Colors: tokens in `frontend/src/app/globals.css` (`--yob-*` for the palette, mapped onto the app's theme tokens).
- In React, use `Logo` and `LogoMark` from `frontend/src/components/shell/logo.tsx`.

## Logo

The mark is a heraldic **yale**: a mythical goat-antelope shown head-on, with its two horns forming a **Y**.
The creature is only the mascot and mark. Never use the word "Yale" in product names, copy or file names.

Colorway: **Gules & Or** (red and gold), with a thin gold inner border (a heraldic *bordure*).

| File | Use |
|---|---|
| `yob-icon.svg` | App icon and avatar, at 48px and up |
| `icon.svg` (in `src/app/`) | Favicons at 32px and below (simplified, no border or small details) |
| `yob-mark-gold.svg` | The gold yale on its own, transparent background (for dark surfaces) |
| `yob-logo-light.svg` | Icon plus wordmark, for light mode (red wordmark) |
| `yob-logo-dark.svg` | Icon plus wordmark, for dark mode (gold wordmark) |
| `yob-wordmark-light.svg` / `-dark.svg` | Wordmark only |
| `png/` | PNG exports: PWA icons (192, 512) and logos (1200 wide); the Apple touch icon is `src/app/apple-icon.png` |

The wordmark is lowercase "yob" in Space Grotesk Bold, converted to outlines, so no font is needed to display the logo.

## Colors

| Token | Hex | Role |
|---|---|---|
| Gules | `#A93127` | Primary brand red: icon tile, light-mode wordmark, primary buttons |
| Or | `#D4A63A` | Gold: the yale, dark-mode wordmark, key highlights |
| Cream | `#F6F0E4` | Light-mode background |
| Charcoal | `#15120F` | Dark-mode background |
| Ink | `#1F1A17` | Light-mode text |
| Parchment | `#FFF8F3` | Dark-mode text |

CSS variables are in `frontend/src/app/globals.css`. Light mode is the default; dark mode sets both `class="dark"` and `data-theme="dark"` on `<html>` (next-themes). In dark mode the primary color is gold on charcoal; in light mode it's red on cream.

## Rules

- The app icon is the same in both modes. Only the wordmark color switches: red on cream, gold on charcoal.
- Use gold sparingly: the logo, the main call to action, and small highlights.
- Do not put gold text on cream. The contrast is too low to read.
- Keep error red (`#E5322D`) brighter than and clearly distinct from the brand red, so the logo never looks like an error.
- Keep success green (`#2E9E5B`) for statuses such as "Applied".
- Typeface: Space Grotesk (Google Fonts) for headings and the UI.
