# SolidClean

Windows için disk alanı analiz ve temizlik aracı. Ücretsiz, GPL-3.0-or-later.

Diller: English, Türkçe, Deutsch, Français, Español, Português (Brasil), Italiano, Русский,
日本語, 简体中文.

> Durum: Faz 4 — Microsoft Store için MSIX paketi. Seçilen öneriler onaydan sonra
> varsayılan olarak Geri Dönüşüm Kutusu'na taşınır; kalıcı silme ayrıca seçilmelidir.
> Yönetici gerektiren adımlar için Windows izin penceresi (UAC) açılır.

## Geliştirme

Gereksinimler: Rust (MSVC), Node.js 20+, Visual Studio Build Tools (C++), WebView2.

```bash
npm install
npm run tauri dev        # uygulamayı açar
npm run dev              # yalnızca arayüz, tarayıcıda sahte veriyle (http://localhost:1420)
```

Testler ve hız ölçümü (`src-tauri` içinde):

```bash
cargo test --release
cargo run --release --bin scanbench -- C:\Users\ad
cargo run --release --bin scanbench -- --cleanup   # önerileri listeler, hiçbir şey silmez
```

Kurulum paketi (NSIS + MSI): `npm run tauri build`

## Microsoft Store (MSIX)

Tauri MSIX üretmediği için paket, Windows SDK araçlarıyla (`makepri`, `makeappx`) elle kurulur:

```bash
npm run msix             # release derler, src-tauri/target/msix/SolidClean_<sürüm>_x64.msix üretir
npm run msix:assets      # ikon değiştiğinde Store/Başlat/görev çubuğu PNG'lerini yeniden üretir
```

- `src-tauri/msix/AppxManifest.xml`: manifest şablonu. AppData yazma sanallaştırması kapalı
  (yoksa diğer uygulamaların önbelleklerini silmek yalnızca paketin sanal katmanında olurdu),
  UAC yardımcısı için `allowElevation` açık. En düşük sürüm Windows 10 2004 (19041).
- `src-tauri/msix/identity.json`: paket kimliği. Store'a yüklemeden önce Partner Center'daki
  "Product identity" değerleriyle değiştirin.
- `store/`: mağaza metinleri (TR/EN), kısıtlı yetenek gerekçeleri, gizlilik politikası.
- `store/screenshots/`: Store ekran görüntüleri. `npm run dev` açıkken
  `./scripts/store-screenshots.ps1` TR ve EN için 3840×2160 PNG'leri `out/` altına üretir.

Yerelde denemek için Windows'ta Geliştirici Modu'nu açıp paketlenmemiş düzeni kaydedin:

```bash
powershell -Command "Add-AppxPackage -Register src-tauri\target\msix\layout\AppxManifest.xml"
```

## Yapı

| Yol | Ne yapar |
|---|---|
| `src-tauri/src/scanner.rs` | Paralel tarayıcı (`FindFirstFileExW`, junction takip etmez, bulut dosyalarını okumaz) |
| `src-tauri/src/category.rs` | Treemap renkleri için kategori tahmini |
| `src-tauri/src/view.rs` | Arayüze gönderilen sınırlı ağaç görünümleri, en büyük dosyalar |
| `src-tauri/src/drives.rs` | Sürücüler, boş alan, küme boyutu |
| `src-tauri/rules.toml` | Temizlik kuralları: ne, neden, nasıl (TR/EN açıklamalarla) |
| `src-tauri/locales/*.toml` | Arka uç metinleri ve kural açıklamalarının diğer dillere çevirisi |
| `src-tauri/src/locale.rs` | Dil seçimi; eksik çeviri testlerde yakalanır |
| `src-tauri/src/cleanup.rs` | Kuralları uygular, boyut ölçer, çalışan uygulamaları bulur |
| `src-tauri/src/clean.rs` | Temizliği yapar: Geri Dönüşüm Kutusu'na taşır ya da kalıcı siler (junction takip etmez) |
| `src-tauri/src/elevate.rs` | Yönetici gerektiren adımları UAC ile ayrı bir yardımcı süreçte çalıştırır |
| `src-tauri/src/history.rs` | Temizlik geçmişi (`%LOCALAPPDATA%\SolidClean\history.json`) |
| `src-tauri/src/safety.rs` | Silme koruması: Windows, Program Files ve kişisel klasörler asla hedef olamaz |
| `src-tauri/src/apps.rs` | Yüklü uygulamalar (Uninstall kayıtları, salt okunur); klasörün sahibini bulur, kaldırma için Ayarlar'ı açar |
| `src/components/Recommendations.tsx` | Öneriler paneli |
| `src/components/CleanupDialog.tsx` | Onay → ilerleme → sonuç penceresi |
| `src/components/UninstallDialog.tsx` | Uygulama klasörü için "Kaldır…": sahibini gösterir, Windows'un kaldırma sayfasına yönlendirir |
| `src/components/Treemap.tsx` | Canvas treemap (squarified), iki seviye iç içe |
| `src/i18n.ts`, `src/locales/` | Arayüz dilleri: Windows dilinden seçilir, üst çubuktan değiştirilebilir |
| `src/mock.ts` | Tarayıcıda arayüz geliştirmek için sahte veri |
