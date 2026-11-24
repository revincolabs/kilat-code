# Design System: Kilat Code (berdasarkan revincolabs.com)

> Referensi: https://revincolabs.com · Diambil: 24 Nov 2026 · Stack referensi: Tailwind CSS v4, tanpa library animasi, light only.
> Diterapkan ke: situs Astro di `website/` (CSS biasa di `src/styles/global.css`, tanpa Tailwind).
> Satu penyesuaian sengaja: aksen **hijau** revincolabs diganti **kuning petir Kilat** (lihat bagian 2). Selain itu sistemnya sama, supaya Kilat terasa dibuat oleh studio yang sama.

## 1. Karakter desain

Tenang, lega, dan percaya diri. Latar abu hangat hampir putih, teks hitam pekat, dan satu warna aksen yang hanya muncul sebagai garis kecil, garis bawah tautan, dan angka penting. Judul besar DM Sans tebal 600 dengan jarak huruf rapat (−0,04em) membawa halaman. Hampir tidak ada ikon, bayangan, atau bingkai: struktur datang dari garis tipis antar-bagian dan pola "judul di kiri, isi di kanan". Pita hitam penuh dipakai dua kali (bukti di atas, ajakan di bawah) untuk ritme.

**Gerakan khas:**
- Tiap bagian dibuka garis tipis selebar layar, lalu grid 12 kolom: kolom kiri (4) berisi penanda `— 01` (garis aksen 20×2px + nomor mono) dan judul H2; kolom kanan (8) berisi teks, daftar, dan kartu.
- Daftar memakai garis aksen pendek (16×2px) sebagai bullet, bukan titik atau ikon.
- Tombol berbentuk pil, hitam pekat dengan panah `→` yang bergeser 4px saat hover. Tautan teks bergaris bawah aksen 2px.
- Label kecil JetBrains Mono huruf kapital dengan jarak lebar (0,14em) untuk kicker, sumber, dan tag.
- Kartu putih bersudut 14px dengan garis 1px, dipakai hemat; kartu unggulan diberi garis aksen 2px dan bayangan lembut berwarna aksen.

**Hindari:**
- Grid kartu ikon, bingkai tebal, sudut tajam di mana-mana (kesan "kotak-kotak").
- Gradient, glow, blur, dan emoji.
- Lebih dari satu warna aksen, atau aksen sebagai blok besar di latar terang.
- Animasi masuk saat scroll; referensi tidak memakainya.

## 2. Warna

| token | hex | peran | dipakai di |
|---|---|---|---|
| `--bg` | `#f4f4f2` | latar halaman | body, header |
| `--bg-subtle` | `#ebebe8` | latar sekunder | kode inline, hover baris |
| `--bg-raised` | `#ffffff` | kartu | kartu, catatan, menu |
| `--fg` | `#181818` | teks utama | judul, tombol |
| `--fg-2` | `#393938` | teks isi | paragraf (referensi: `--fg` 85%) |
| `--fg-muted` | `#5f5f5c` | teks pelan | lead, nav, keterangan |
| `--border` | `#dededa` | garis tipis | pemisah bagian, kartu |
| `--border-strong` | `#c4c4c2` | garis tegas | tombol garis, tag, lingkaran FAQ |
| `--ink` | `#181818` | pita gelap | bukti, ajakan, footer |
| `--ink-raised` | `#222222` | kartu di pita gelap | bingkai screenshot |
| `--ink-fg` | `#f4f4f2` | teks di pita gelap | |
| `--ink-muted` | `#a8a8a4` | teks pelan di pita gelap | |
| `--ink-line` | `#383836` | garis di pita gelap | |
| `--brand` | `#8a5c00` | teks aksen (Kilat) | tautan "→", nomor langkah, focus ring. Referensi: `#15803d` |
| `--accent` | `#f5b70f` | garis aksen (Kilat) | penanda `—`, bullet, garis bawah tautan, kartu unggulan. Referensi: `#16a34a` |
| `--accent-soft` | `#fff3cc` | latar aksen lembut | catatan, tag "sistem kamu". Referensi: `#e6f4ea` |
| `--ink-accent` | `#ffc83d` | aksen di pita gelap | angka besar, hover tombol terang. Referensi: `#4ade80` |
| `--warn` | `#b45309` | peringatan | status "kena limit", catatan bahaya |

- Tanpa gradient (referensi: 0). Light only; referensi tidak punya mode gelap.
- `::selection`: latar `--ink-accent`, teks `--fg`.
- Kuning tidak pernah jadi warna teks di latar terang (kontras rendah); untuk teks pakai `--brand` (kontras 5,3:1 di `--bg`).

## 3. Tipografi

- **DM Sans Variable** (Google Fonts, OFL), bobot 400/500/600. **JetBrains Mono Variable** (OFL) untuk label. Keduanya di-host sendiri di `public/fonts/` (subset latin + latin-ext), `font-display: swap`.
- Skala fluid (dari token referensi), angka di tabel = nilai pada lebar 1440 / 390.

| style | desktop | mobile | bobot | line-height | letter-spacing | dipakai untuk |
|---|---|---|---|---|---|---|
| display `--step-display` | 92px | 43,5px | 600 | 0,98 | −0,04em | H1 hero |
| h2-xl `--step-4` | 61px | 34px | 600 | 1,0 | −0,04em | judul pita ajakan, H1 dokumentasi |
| h2 `--step-3` | 43px | 27,5px | 600 | 1,08 | −0,03em | judul bagian |
| lead `--step-2` | 29px | 22px | 400 | 1,375 | 0 | lead hero, judul kartu besar |
| h3 `--step-1` | 23px | 19px | 600 | 1,3 | −0,01em | judul kartu, pertanyaan FAQ |
| body `--step-0` | 18,6px | 16,5px | 400 | 1,7 | 0 | paragraf |
| small | 15px | 15px | 400–600 | 1,55 | 0 | nav, tombol, keterangan |
| label | 12px | 12px | 400 | 1,55 | 0,14em, kapital | kicker, sumber, tag (mono) |

Aturan: panjang baris maks `68ch`; judul huruf kalimat biasa (bukan kapital semua); penekanan dengan `<b>`/600, bukan warna; `text-wrap: balance` di judul.

## 4. Spasi dan ukuran

- Unit dasar 4px. Skala yang dipakai: 8, 12, 16, 20, 24, 32, 40, 48, 56, 80, 96.
- Ritme bagian: 96px atas/bawah di desktop, 64px di mobile (perkiraan dari screenshot).
- Kartu: padding 32px (desktop) / 24px. Tombol: tinggi 44px (`--tap`), padding samping 24px.
- Gap: grid utama 40px; kartu berdampingan 16–20px; daftar 16px.

## 5. Layout

- Container maks 1280px; gutter 20px (<768), 32px (768–1279), 40px (≥1280). Isi efektif 1200px.
- Grid 12 kolom, gap 40px. Kolom samping 4 kolom (judul menempel `position: sticky` saat di-scroll), isi 8 kolom.
- Breakpoint: 640, 768, 1024, 1280. Di bawah 1024 grid jadi satu kolom.

Anatomi beranda Kilat:
1. Header menempel, 76px, garis bawah tipis. Logo kiri, nav tengah, GitHub (mono) + tombol pil kanan.
2. Hero: H1 dua baris, lead abu, tombol pil + tautan bergaris bawah, baris meta mono. Padding atas ±100px.
3. Pita gelap "bukti": kartu `--ink-raised` berisi screenshot aplikasi, garis, keterangan mono; di bawahnya tiga fakta dengan angka kuning besar.
4. Bagian bernomor 01–08 (judul kiri, isi kanan), dipisah garis tipis selebar layar.
5. Bagian Download memakai varian lebar: judul di atas, tiga kartu platform selebar container.
6. Pita gelap ajakan + footer gelap.

Z-index: header 40, menu mobile 50.

## 6. Bentuk dan kedalaman

- Radius: 14px (`--radius`, kartu dan bingkai screenshot), 8px (`--radius-sm`, blok kode, gambar di dalam kartu), pil 999px (tombol, tag, lingkaran FAQ).
- Garis: 1px `--border`; 1px `--border-strong` untuk kontrol; 2px `--accent` hanya untuk kartu unggulan.
- Bayangan: hanya dua, keduanya lunak dan jatuh ke bawah: menu `0 12px 32px -12px rgb(0 0 0 / .25)`, kartu unggulan `0 18px 40px -24px rgb(245 183 15 / .55)`.
- Tanpa blur atau efek kaca.

## 7. Komponen

**Tombol** (pil, 44px, 15px/600, gap 8px, transisi warna 150ms):
- `btn-ink`: latar `--ink`, teks `--ink-fg`; hover latar `#2e2e2c`. Panah `→` bergeser 4px saat hover.
- `btn-line`: transparan, garis 1px `--border-strong`; hover garis `--fg`.
- `btn-light` (di pita gelap): latar `--ink-fg`, teks `--ink`; hover latar `--ink-accent`.
- Fokus: outline 2px `--brand`, offset 3px. Disabled tidak dipakai.

**Tautan teks**: 15px/600, garis bawah 2px `--accent`, offset 6px; hover warna `--brand`. Tautan aksen ("Baca selengkapnya →"): warna `--brand`, tanpa garis bawah, panah bergeser.

**Navbar**: link 15px `--fg-muted`, gap 32px, hover/aktif `--fg`. Di bawah 1024px diganti tombol "Menu" + dua garis; panel menu putih di bawah header.

**Kicker** `— 01`: garis 20×2px `--accent` + teks mono 12px kapital 0,14em `--fg-muted`.

**Kartu**: latar putih, garis 1px `--border`, radius 14px, padding 32px. Unggulan: garis 2px `--accent` + bayangan aksen.

**Tag**: pil, garis 1px `--border-strong`, mono 11px kapital 0,08em, padding 3px 10px, `--fg-muted`.

**FAQ**: baris dengan garis atas/bawah `--border`, pertanyaan 23px/600, lingkaran 28px bergaris `--border-strong` berisi `+` warna `--brand` (jadi `−` saat terbuka).

**Footer**: pita `--ink`; logo + tagline, kolom tautan dengan judul mono kapital `--ink-muted`, tautan 15px `--ink-fg` 85% (hover `--ink-accent`); baris bawah © dan disclaimer 13px.

## 8. Gerak dan animasi

Hemat dan cepat: hanya umpan balik hover/fokus, tidak ada animasi masuk, parallax, atau smooth-scroll library.

| token | nilai | dipakai |
|---|---|---|
| `--dur` | 150ms | semua transisi warna, garis, geser panah |
| `--ease` | `cubic-bezier(.4, 0, .2, 1)` | default |
| `--ease-out` | `cubic-bezier(.16, 1, .3, 1)` | buka menu/FAQ (jika dianimasikan) |

- Hover tombol: warna latar 150ms; panah `translate: 4px 0`.
- Hover tautan: warna teks ke `--brand`.
- Header tidak berubah saat scroll (tetap menempel, tanpa bayangan).
- `prefers-reduced-motion: reduce`: semua durasi transisi/animasi jadi 0,01ms.

## 9. Gambar dan ikon

- Referensi tidak memakai foto atau ilustrasi. Kilat memakai **screenshot asli aplikasi** dalam bingkai radius 14px (gambar di dalam kartu: radius 8px).
- Ikon hampir tidak ada: panah `→` teks, `+`/`−` di FAQ, centang kecil di lingkaran. Tidak ada ikon dekoratif.

## 10. Responsif

- <1280: gutter 32px. <1024: grid bagian satu kolom (judul di atas isi, sticky mati), nav jadi tombol Menu, kartu download satu kolom. <768: gutter 20px, padding bagian 64px. <640: tombol hero selebar layar, tombol Download di header disembunyikan.
- Skala huruf turun otomatis lewat `clamp()`.

## 11. Token

```css
:root {
  --bg: #f4f4f2;
  --bg-subtle: #ebebe8;
  --bg-raised: #ffffff;
  --fg: #181818;
  --fg-2: #393938;
  --fg-muted: #5f5f5c;
  --border: #dededa;
  --border-strong: #c4c4c2;
  --ink: #181818;
  --ink-raised: #222222;
  --ink-fg: #f4f4f2;
  --ink-fg-2: #d3d3d1;
  --ink-muted: #a8a8a4;
  --ink-line: #383836;
  --brand: #8a5c00;
  --accent: #f5b70f;
  --accent-soft: #fff3cc;
  --ink-accent: #ffc83d;
  --warn: #b45309;
  --font-sans: "DM Sans Variable", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "JetBrains Mono Variable", ui-monospace, "SFMono-Regular", monospace;
  --step--1: clamp(0.83rem, 0.79rem + 0.17vw, 0.98rem);
  --step-0: clamp(1rem, 0.94rem + 0.25vw, 1.24rem);
  --step-1: clamp(1.15rem, 1.05rem + 0.42vw, 1.55rem);
  --step-2: clamp(1.35rem, 1.2rem + 0.68vw, 2rem);
  --step-3: clamp(1.7rem, 1.35rem + 1.5vw, 3.1rem);
  --step-4: clamp(2.1rem, 1.5rem + 2.6vw, 4.4rem);
  --step-display: clamp(2.6rem, 1.6rem + 4.6vw, 6.8rem);
  --tracking-display: -0.04em;
  --tracking-heading: -0.03em;
  --measure: 68ch;
  --radius: 14px;
  --radius-sm: 8px;
  --tap: 44px;
  --dur: 150ms;
  --ease: cubic-bezier(0.4, 0, 0.2, 1);
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --max: 1280px;
}
```

## 12. Checklist

1. Tidak ada gradient, glow, blur, emoji, atau grid kartu ikon.
2. Setiap bagian dibuka garis tipis selebar layar dan penanda `— NN`.
3. Judul bagian di kolom kiri (4/12), isi di kanan (8/12); di bawah 1024px satu kolom.
4. Semua tombol berbentuk pil setinggi 44px; tombol utama hitam dengan panah `→`.
5. Kuning hanya sebagai garis/aksen kecil di latar terang, dan sebagai angka/hover di pita gelap. Teks aksen memakai `--brand`.
6. Kartu putih radius 14px dengan garis 1px; paling banyak satu kartu unggulan per kelompok.
7. Judul DM Sans 600 dengan tracking negatif; label JetBrains Mono kapital 12px.
8. Paragraf maksimal `68ch`, line-height 1,7.
9. Screenshot di bingkai radius 14px; tidak ada mockup gambar tangan.
10. Pita gelap hanya untuk bukti (atas) dan ajakan + footer (bawah).
11. Transisi 150ms, tidak ada animasi masuk; reduced-motion mematikan semuanya.
12. Lebar 390px: tidak ada scroll horizontal, tombol hero selebar layar.
