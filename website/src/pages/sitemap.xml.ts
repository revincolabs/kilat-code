// sitemap.xml with hreflang alternates for every page in every language.
import type { APIRoute } from "astro";
import { DOCS_UPDATED, SITE_URL } from "../site";
import { LANGS, X_DEFAULT, localePath } from "../i18n";

const PAGES = [
  { path: "/", lastmod: DOCS_UPDATED },
  { path: "/docs", lastmod: DOCS_UPDATED },
];

export const GET: APIRoute = () => {
  const abs = (p: string) => new URL(p, SITE_URL).href;
  const urls = PAGES.flatMap(({ path, lastmod }) =>
    LANGS.map((lang) => {
      const alts = [
        ...LANGS.map((l) => `    <xhtml:link rel="alternate" hreflang="${l}" href="${abs(localePath(l, path))}"/>`),
        `    <xhtml:link rel="alternate" hreflang="x-default" href="${abs(localePath(X_DEFAULT, path))}"/>`,
      ].join("\n");
      return `  <url>\n    <loc>${abs(localePath(lang, path))}</loc>\n    <lastmod>${lastmod}</lastmod>\n${alts}\n  </url>`;
    }),
  );
  const xml = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${urls.join("\n")}
</urlset>
`;
  return new Response(xml, { headers: { "Content-Type": "application/xml; charset=utf-8" } });
};
