// @ts-check
import { defineConfig } from "astro/config";
import mdx from "@astrojs/mdx";
import { SITE_URL } from "./src/site.ts";

export default defineConfig({
  site: SITE_URL,
  integrations: [mdx()],
  // Pure static output: Vercel serves the files from dist/.
  output: "static",
  // Clean URLs without a trailing slash: /docs, /en, /en/docs (see vercel.json).
  trailingSlash: "never",
  build: { format: "file" },
});
