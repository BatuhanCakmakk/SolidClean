# SolidClean: Microsoft Store başvuru metinleri

Partner Center'daki alanlara kopyalanacak metinler. Her dil için ayrı "Store listing" oluşturun
(Türkçe ve İngilizce).

## Genel

| Alan | Değer |
|---|---|
| Kategori | Utilities & tools |
| Fiyat | Ücretsiz |
| Yaş sınıflandırması | Anket: kullanıcı içeriği yok, satın alma yok, konum yok, kişisel veri toplanmıyor → herkese uygun |
| Gizlilik politikası URL'si | `store/privacy.md` dosyasını yayınlayıp (ör. GitHub Pages) adresini yazın |
| Destek | GitHub Issues sayfası veya bir e-posta adresi |
| Ekran görüntüleri | En az 1, önerilen 4 adet, 1366×768 veya üstü. Aşağıdaki "Ekran görüntüleri" bölümüne bakın. |

## Türkçe (tr-TR)

**Kısa açıklama**

Diskinizde neyin yer kapladığını görün, önbellek ve geçici dosyaları güvenle temizleyin.

**Açıklama**

SolidClean, diskinizdeki alanı kutularla gösteren bir harita çizer: büyük kutu, büyük klasör.
Neyin yer kapladığını birkaç saniyede görürsünüz.

Tarama bittiğinde SolidClean, silinmesi güvenli olanları ve önce gözden geçirmeniz
gerekenleri ayrı ayrı listeler. Her önerinin ne olduğunu, silinirse ne olacağını ve
kendiliğinden yeniden oluşup oluşmadığını açıkça yazar.

Temizlik varsayılan olarak Geri Dönüşüm Kutusu'na taşıma şeklinde yapılır; yanlışlıkla
silinen bir şeyi geri alabilirsiniz. Windows, Program Files ve kişisel klasörleriniz
(Belgeler, Resimler, Masaüstü…) hiçbir zaman silinmez.

SolidClean ücretsizdir, açık kaynaklıdır (GPL-3.0) ve hiçbir veriyi bilgisayarınızdan dışarı göndermez.

**Özellikler**

- Hızlı tarama: yüz binlerce dosya saniyeler içinde
- Klasörleri boyutuna göre gösteren etkileşimli harita
- "Güvenle silinebilir" ve "Önce gözden geçirin" diye ayrılmış öneriler
- Tarayıcı, Windows Update, geliştirici araçları ve uygulama önbellekleri
- Geri Dönüşüm Kutusu'na taşıyarak geri alınabilir temizlik
- Açık olan uygulamaların dosyalarına dokunmaz
- Ollama, Hugging Face, Docker gibi büyük yapay zekâ ve geliştirici dosyalarını bulur
- En büyük dosyalar listesi ve temizlik geçmişi
- OneDrive'da yalnızca bulutta duran dosyaları indirmez, diskte yer kaplıyormuş gibi saymaz
- Reklam yok, hesap yok, veri toplama yok

**Arama terimleri** (en fazla 7)

disk temizleme, disk alanı, önbellek temizle, geçici dosyalar, depolama, treemap, temizlik

## English (en-US)

**Short description**

See what fills your disk and safely clean caches and temporary files.

**Description**

SolidClean draws a map of your disk where every box is a folder, sized by the space it takes.
You see what is filling your drive in seconds.

After a scan, SolidClean lists what is safe to remove and what you should review first.
Each recommendation explains what it is, what happens if you remove it, and whether it
comes back on its own.

Cleanup moves items to the Recycle Bin by default, so you can restore anything removed by
mistake. Windows, Program Files and your personal folders (Documents, Pictures, Desktop…)
are never touched.

SolidClean is free, open source (GPL-3.0) and never sends any data off your PC.

**Features**

- Fast scanning: hundreds of thousands of files in seconds
- Interactive map of folders sized by space used
- Recommendations split into "Safe to remove" and "Review first"
- Browser, Windows Update, developer tool and app caches
- Undoable cleanup through the Recycle Bin
- Skips files of apps that are still open
- Finds large AI and developer files such as Ollama, Hugging Face and Docker
- Largest files list and cleanup history
- Never downloads OneDrive cloud-only files or counts them as used disk space
- No ads, no account, no data collection

**Search terms** (up to 7)

disk cleanup, disk space, storage, cache cleaner, temp files, treemap, cleaner

## Ekran görüntüleri

Hazır görseller: `store/screenshots/out/tr/` (Türkçe listeleme) ve `store/screenshots/out/en/`
(İngilizce listeleme). 3840×2160 PNG; Store en az 1366×768 ister, 4K'ya kadar kabul eder.
Partner Center'a bu sırayla yükleyin:

| # | Dosya | Başlık (TR / EN) |
|---|---|---|
| 1 | `01-hero.png` | GÖR. ANLA. TEMİZLE. / SEE IT. KNOW IT. CLEAN IT. |
| 2 | `02-map.png` | NE YER KAPLIYOR, BİR BAKIŞTA / EVERY GIGABYTE AT A GLANCE |
| 3 | `03-explain.png` | NEYİ SİLDİĞİNİZİ BİLİN / KNOW WHAT YOU DELETE |
| 4 | `04-safety.png` | ÖNCE GÜVENLİK / SAFETY FIRST |
| 5 | `05-apps.png` | UYGULAMALARI DOĞRU YOLDAN KALDIRIN / REMOVE APPS THE RIGHT WAY |
| 6 | `06-theme.png` | GECE YA DA GÜNDÜZ / DAY OR NIGHT |

Her görselin "açıklama" alanına başlığı yazın. Görseller sahte veriyle (`src/mock.ts`) üretilir,
kişisel dosya adı içermez. Yeniden üretmek için:

1. `npm run dev`
2. `./scripts/store-screenshots.ps1` (yalnızca biri için: `-Slides hero -Langs en`)

Metinler ve sahneler `store/screenshots/index.html` içinde; tarayıcıda
`http://localhost:1420/store/screenshots/?slide=hero&lang=tr` ile önizlenir.

## Kısıtlı yetenekler (Submission options → Restricted capabilities)

Pakette üç kısıtlı yetenek var. Partner Center her biri için gerekçe ister; aşağıdaki
İngilizce metinleri kullanın.

**runFullTrust**

> SolidClean is a Win32 desktop app (Rust + WebView2). It scans the file system with Win32
> APIs and needs full trust to read folder sizes outside its package.

**unvirtualizedResources** (`FileSystemWriteVirtualization=disabled`)

> SolidClean is a disk cleanup tool. Most of the caches it cleans live in other apps'
> folders under %LOCALAPPDATA% and %APPDATA% (browser caches, crash dumps, npm/pip caches,
> %LOCALAPPDATA%\Temp). With AppData write virtualization enabled, deleting those files would
> only hide them inside the package's private overlay while the real files stay on disk, so
> the cleanup would report freed space that was never freed. Disabling write virtualization
> makes deletions act on the real files. SolidClean only deletes paths the user has explicitly
> confirmed, moves them to the Recycle Bin by default, and never deletes Windows, Program Files
> or the user's personal folders.

**allowElevation**

> A few optional cleanup steps need administrator rights: C:\Windows\Temp, Windows Update
> download cache, Delivery Optimization cache, system error reports, memory dumps and
> turning off hibernation. For these, SolidClean starts a short-lived helper of its own executable through
> the standard UAC prompt, only after the user selects these items and confirms. The main app
> always runs without elevation; if the user declines UAC, those steps are skipped.

## Notlar

- Paket imzasız yüklenebilir; Store yayın sırasında Microsoft sertifikasıyla imzalar.
- Sürüm `tauri.conf.json` içindeki `version` + `.0` olur (ör. 0.1.0 → 0.1.0.0). Her yeni
  gönderimde sürümü artırın.
- Paketteki kimlik değerleri (`src-tauri/msix/identity.json`) Partner Center'daki
  "Product identity" sayfasıyla birebir aynı olmalı, yoksa yükleme reddedilir.
