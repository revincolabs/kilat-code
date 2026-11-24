# Design System: Kilat Code (website)

> Inspirasi: https://revincolabs.com (studio pembuat Kilat), diambil 24 Nov 2026. Kilat sengaja **tidak** menyalinnya: yang diambil hanya rasa tenang, lega, dan percaya diri. Ciri khas revincolabs (abu `#f4f4f2`, DM Sans, tombol pil hitam, penanda "— 01" dengan judul kiri/isi kanan, pita hitam selebar layar, footer hitam) diganti dengan ciri Kilat di bawah.
> Diterapkan di: `src/styles/global.css` (CSS biasa, tanpa Tailwind).

## 1. Karakter

Putih bersih, huruf grotesk tegas, dan kuning petir Kilat sebagai warna merek yang benar-benar dipakai: tombol utama, garis bawah judul hero, panel ajakan. Produk tampil lewat screenshot asli di panel gelap. Struktur datang dari judul besar dan jarak lega, bukan dari kotak bergaris.

**Ciri khas Kilat:**
- Label bagian = ikon petir kuning kecil + teks mono kapital (mis. `⚡ SAAT KENA LIMIT`), bukan nomor.
- Judul bagian besar di atas, lalu isi **berdampingan**: teks (5/12) dan visual (7/12).
- Tombol bersudut 10px; tombol utama **kuning** dengan teks hitam. Ikon GitHub asli di navbar dan di samping tombol download.
- Kartu **berisi abu muda tanpa garis** (`--bg-soft`), bukan kartu putih bergaris. Kartu unggulan berisi kuning muda.
- Panel gelap dan panel kuning bersudut 24px di dalam container (tidak selebar layar).
- Daftar sebagai baris bergaris tipis (judul tebal + teks pelan), bukan bullet.

**Hindari:** gradient, glow, blur, emoji, grid kartu ikon, sudut tajam di mana-mana, tombol pil hitam dan penanda "— 01" (milik revincolabs), mode gelap halaman.

## 2. Warna

| token | hex | peran |
|---|---|---|
| `--bg` | `#ffffff` | latar halaman |
| `--bg-soft` | `#f5f4f0` | kartu berisi, kode inline, hover nav |
| `--fg` | `#141412` | teks utama, tombol gelap |
| `--fg-2` | `#3a3a36` | paragraf |
| `--fg-muted` | `#6a6964` | teks pelan, label |
| `--line` / `--line-strong` | `#e6e5df` / `#cfcdc5` | garis tipis / garis kontrol |
| `--panel` | `#0f0f0e` | panel gelap (sama dengan latar aplikasi) |
| `--panel-fg` / `--panel-muted` / `--panel-line` | `#f2f1ec` / `#9d9b93` / `#2c2c29` | isi panel gelap |
| `--yellow` | `#ffc83d` | tombol utama, panel ajakan, garis bawah hero, angka di panel gelap |
| `--yellow-hover` | `#ffd567` | hover tombol kuning |
| `--yellow-deep` | `#eba900` | ikon petir, hover garis bawah tautan |
| `--yellow-soft` | `#fff4d1` | kartu unggulan, catatan, item daftar isi aktif |
| `--brand-text` | `#8a5c00` | teks aksen (kontras 5,3:1), focus ring |
| `--warn` | `#b45309` | status "kena limit" |

Kuning tidak pernah jadi warna teks di latar putih.

## 3. Tipografi

- **Instrument Sans Variable** (OFL, bobot 400–700) untuk semua teks; **JetBrains Mono Variable** untuk label, nomor, ukuran file. Di-host sendiri di `public/fonts/`.

| style | token | ukuran 1440 / 390 | bobot | line-height | tracking |
|---|---|---|---|---|---|
| display | `--text-display` | ±99px / 44px | 600 | 0,96 | −0,035em |
| h2 | `--text-h2` | ±54px / 32px | 600 | 1,02 | −0,03em |
| h3 | `--text-h3` | ±22px / 18,5px | 600 | 1,3 | −0,015em |
| lead | `--text-lead` | ±25px / 19px | 400 | 1,4–1,45 | 0 |
| body | `--text-body` | ±18px / 16px | 400 | 1,6–1,65 | 0 |
| label | — | 12px | 500 mono | 1,5 | 0,12em, kapital |

Judul hero: baris kedua diberi garis bawah kuning tebal (`text-decoration-thickness: .12em`).

## 4. Spasi dan layout

- Container 1240px, gutter 32px (20px di bawah 768). Jarak antar-bagian `--block-y`: 128 / 104 / 88px.
- Bagian: label → h2 → lead opsional (maks 880px), lalu `.split` (5fr/7fr, gap 56px) atau konten selebar container.
- Hero: h1 dua baris, garis tipis, lalu lead (7fr) dan tombol + meta (5fr) sejajar bawah; lalu panel gelap berisi screenshot dan tiga fakta.
- Breakpoint: 1023 (nav jadi menu, split jadi satu kolom), 767 (gutter kecil, fakta bertumpuk), 639 (tombol hero selebar layar).

## 5. Bentuk

Radius 24px (panel gelap dan kuning), 16px (kartu, bingkai screenshot), 10px (tombol, menu), 6–8px (tag, kode). Tanpa bayangan kecuali menu mobile (`0 16px 40px -16px rgb(0 0 0 / .25)`).

## 6. Komponen

- **Tombol** 44px (`.btn-sm` 40px), sudut 10px, 15px/600: `btn-yellow` (utama), `btn-line` (putih bergaris), `btn-dark` (di panel kuning). Download memakai ikon panah-bawah, GitHub memakai logo GitHub.
- **Navbar**: logo kiri; tautan kanan dengan hover berlatar abu; garis pemisah; ikon GitHub; tombol Download kuning. Di bawah 1024px: ikon GitHub + tombol menu (ikon garis ↔ silang).
- **Label**: petir kuning 12px + mono kapital.
- **Baris** (`.rows`): garis atas/bawah tipis, judul tebal 17px, teks pelan.
- **Trio**: tiga pernyataan berdampingan dengan garis atas hitam dan nomor mono.
- **Kartu berisi** (`.soft`, `.steps li`, `.dl-card`): `--bg-soft`, radius 16px, tanpa garis. Nomor langkah dalam kotak kuning 32px.
- **FAQ**: garis atas hitam, baris bergaris tipis, `+` mono yang berputar jadi `×`.
- **Footer**: terang, garis atas, empat kolom, baris bawah disclaimer + © mono.

## 7. Gerak

Hanya hover dan fokus, 160ms `cubic-bezier(.4,0,.2,1)`: warna latar tombol, garis bawah tautan, panah `→` di tautan `.more` bergeser 3px, `+` FAQ berputar 45°. Tanpa animasi masuk. `prefers-reduced-motion` mematikan semuanya.

## 8. Checklist

1. Tidak ada gradient, glow, blur, emoji, atau grid kartu ikon.
2. Tidak memakai ciri revincolabs: latar abu, tombol pil hitam, penanda "— 01", pita hitam selebar layar.
3. Tombol utama kuning bersudut 10px; GitHub selalu dengan logonya.
4. Setiap bagian: label petir, judul besar, lalu isi (teks dan visual berdampingan bila ada visual).
5. Kartu berisi abu muda tanpa garis; paling banyak satu kartu kuning muda per kelompok.
6. Screenshot asli di bingkai radius 16px atau di panel gelap.
7. Lebar 390px: tanpa scroll horizontal.
