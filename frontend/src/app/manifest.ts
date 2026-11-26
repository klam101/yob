import type { MetadataRoute } from "next";

/** Lets the browser install yob as an app, with the brand icon and colors. */
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: "yob",
    short_name: "yob",
    description: "Personal job search: one feed of fresh jobs, tailored documents, autofill.",
    start_url: "/jobs",
    display: "standalone",
    background_color: "#f6f0e4",
    theme_color: "#a93127",
    icons: [
      { src: "/brand/png/yob-icon-192.png", sizes: "192x192", type: "image/png" },
      { src: "/brand/png/yob-icon-512.png", sizes: "512x512", type: "image/png" },
    ],
  };
}
