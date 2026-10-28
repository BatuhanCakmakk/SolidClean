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
- Tarayıcı, geliştirici araçları ve uygulama önbellekleri; Windows Update gibi sistem
  dosyaları için Ayarlar'da izlenecek yol
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
- Browser, developer tool and app caches, plus the steps in Settings for system files
  such as Windows Update
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

## Store logoları

`./scripts/store-screenshots.ps1 -Art` ile üretilir (`store/art/out/`), iki dilde aynıdır:

- 9:16 Poster art: `poster-art.png` (1440×2160)
- 1:1 Box art: `box-art.png` (2160×2160)

Alttaki 300×300 / 150×150 / 71×71 alanlarını boş bırakın; Store paketteki ikonları kullanır.

## Kısıtlı yetenekler (Submission options → Restricted capabilities)

Pakette yalnızca `runFullTrust` var. Partner Center gerekçe isterse:

> SolidClean is a Win32 desktop app (Rust + WebView2). It scans the file system with Win32
> APIs and needs full trust to read folder sizes outside its package.

İlk gönderimde (0.1.0) istenen `allowElevation` ve `unvirtualizedResources` 10.6.3 gereği
reddedildi ve 0.1.1'de kaldırıldı:

- Yönetici gerektiren kurallar (Windows\Temp, Windows Update, Teslim İyileştirme, sistem
  hata raporları, bellek dökümleri, Windows.old) artık yalnızca ölçülür;
  temizlik için Windows Ayarlar'ındaki yol gösterilir. `rules.toml`'da `admin = true` olan
  her kuralın `manual` metni olması testle zorunlu.
- AppData yazma sanallaştırması açık. Windows 10 1903'ten beri paketli uygulamaların var
  olan AppData dosyalarını silmesi gerçek konumda olur; yalnızca paketin yeni oluşturduğu
  dosyalar özel konuma yönlendirilir. En düşük sürüm 19041 olduğu için bu yeterli.

Gerekçe alanlarından bu ikisini silin; yoksa paketle uyuşmaz.

## Notes for certification

> No account or sign-in is needed. Click "Scan" to analyze the system drive; the Recommendations
> tab lists what can be cleaned, with an explanation of each item. Selected items are moved to
> the Recycle Bin by default after a confirmation dialog. SolidClean never asks for administrator
> rights: items that need them are shown with instructions for Windows Settings and cannot be
> selected. The "Support" link in the ⋯ menu opens an optional donation page in the browser and
> unlocks nothing. Source code: https://github.com/BatuhanCakmakk/SolidClean

## Notlar

- Paket imzasız yüklenebilir; Store yayın sırasında Microsoft sertifikasıyla imzalar.
- Sürüm `tauri.conf.json` içindeki `version` + `.0` olur (ör. 0.1.1 → 0.1.1.0). Her yeni
  gönderimde sürümü artırın.
- Paketteki kimlik değerleri (`src-tauri/msix/identity.json`) Partner Center'daki
  "Product identity" sayfasıyla birebir aynı olmalı, yoksa yükleme reddedilir.
