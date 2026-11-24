# Situs Kilat Code

Astro (output statis) di Vercel. Live: https://kilat-code.vercel.app

```
src/site.ts                  versi, nama file installer, alamat situs  ← ubah di sini saat rilis
src/pages/index.astro        halaman utama (fitur, saat kena limit, mulai, FAQ)
src/pages/docs.astro         halaman dokumentasi + daftar isi otomatis
src/docs/docs.mdx            ISI dokumentasi (Markdown/MDX)
src/components/              Header, Footer, AppMockup, DownloadButton, DownloadBox, Note
src/styles/global.css        semua gaya
public/icon.png              logo / favicon
public/downloads/            installer yang bisa diunduh
```

## Pengembangan

```bash
pnpm install
pnpm dev        # http://localhost:4321
pnpm build      # hasil di dist/
```

Daftar isi dokumentasi dibuat dari judul `##` di `docs.mdx`. Grup di daftar isi (Mulai, Memakai, …) diatur di `groupStarts` pada `src/pages/docs.astro`. Kalau judul diganti, slug-nya ikut berubah, jadi cek tautan `/docs#…`.

## Rilis versi baru

1. `pnpm tauri build` di root project.
2. Salin `src-tauri/target/release/bundle/nsis/Kilat Code_<versi>_x64-setup.exe` ke `public/downloads/KilatCode-<versi>-x64-setup.exe` (hapus yang lama kalau tidak perlu).
3. Ubah `VERSION` di `src/site.ts`.
4. Deploy (lihat di bawah).

## Deploy (Vercel)

Folder ini sudah terhubung ke project Vercel `kilat-code` (`.vercel/`).

```bash
vercel deploy          # preview (URL terpisah, untuk dicek dulu)
vercel deploy --prod   # production → kilat-code.vercel.app
```

### Pasang subdomain sendiri

1. `vercel domains add <sub>.revincolabs.<tld> kilat-code` (atau lewat Dashboard → Project → Settings → Domains).
2. Di DNS domain revincolabs, tambahkan record `CNAME <sub>` → `cname.vercel-dns.com`.
3. Ubah `SITE_URL` di `src/site.ts` ke alamat baru, lalu `vercel deploy --prod`.
