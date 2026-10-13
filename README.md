# Privora Rust — Offline Alpha 0.3

**یک نمونهٔ اولیهٔ واقعی برای ساخت بسته‌های رمزگذاری‌شدهٔ `.vora` با Rust**

این ریپو رابط دسکتاپ Native با `egui/eframe` و هستهٔ مستقل با Rust دارد. رابط از کانسپت تصویری Privora الهام گرفته است: تم تیره، رنگ بنفش، نوار کناری و صفحات Home، Create Package، Licenses، Files / Unlock و Settings.

> **وضعیت:** آلفای دسکتاپ v0.3 با فرمت `.vora` v2. این آرشیو در محیطی که Rust/Cargo در دسترس نیست از نظر ساختار بررسی شده، اما کامپایل نهایی این نسخه اینجا اجرا نشده است. قبل از استفاده برای داده‌های حساس، فرمان‌های تست را روی سیستم توسعه اجرا کن و برای انتشار عمومی بازبینی امنیتی مستقل بگیر.

## قابلیت‌های پیاده‌سازی‌شده

- انتخاب چند فایل و بسته‌بندی آن‌ها در یک فایل `.vora`.
- پشتیبانی بایت‌محور از فایل‌های متنی، تصویری، صوتی، ویدیویی و فرمت‌های دیگر.
- رمزگذاری قطعه‌ای AES-256-GCM با احراز اصالت هر قطعه.
- تولید خودکار یک کد دسترسی تصادفی ۲۵۶ بیتی.
- مشتق‌سازی کلید با Argon2id؛ کد دسترسی خام داخل `.vora` ذخیره نمی‌شود.
- در `.vora` v2، کل Manifest شامل شناسه/نام بسته، زمان ساخت، شروع اعتبار، انقضا، برچسب گیرنده، یادداشت محرمانه، سقف بازکردن، تعداد/حجم فایل‌ها و برچسب‌های رمزنگاری به‌صورت مستقل رمزگذاری می‌شود؛ فقط magic، salt تصادفی، nonce تصادفی، طول Manifest رمز‌شده و اندازهٔ تقریبی بسته قابل مشاهده‌اند. همچنین خواندن بسته‌های legacy v1 حفظ شده است.
- کد دسترسی پس از ساخت نمایش داده می‌شود و دکمهٔ Copy دارد؛ فایل `.license.txt` دیگر به‌شکل خودکار کنار بسته ساخته نمی‌شود تا راز دسترسی ناخواسته همراه فایل ارسال نشود. در صورت نیاز می‌توان یادداشت مجوز را جداگانه ذخیره کرد؛ این یادداشت حاوی کد plaintext است.
- واردکردن بسته، بررسی تاریخ شروع اعتبار و انقضا، بررسی شمارندهٔ محلی و استخراج امن فایل‌ها.
- تنظیم optional: recipient label، یادداشت محرمانهٔ داخل Manifest رمزگذاری‌شده، شروع اعتبار، تاریخ انقضا و سقف بازکردن محلی.
- در تاریخچهٔ محلی، متن یادداشت محرمانه ذخیره نمی‌شود؛ فقط پرچم وجود یادداشت نگهداری می‌شود.
- جلوگیری از استخراج مسیرهای ZIP مشکوک و جلوگیری از overwrite فایل‌های هم‌نام در پوشهٔ خروجی.
- تاریخچهٔ محلی بسته‌ها و بازکردن‌ها در پروفایل برنامه.
- تست‌های خودکار برای round-trip تک‌فایل و چندقطعه‌ای، رد کد اشتباه، دست‌کاری Manifest رمز‌شده و بررسی نبودن metadata مشخص در بایت‌های `.vora` جدید.

## نیازمندی‌ها

- Rust و Cargo نسخهٔ **1.95 یا جدیدتر** (به‌دلیل نسخهٔ انتخاب‌شدهٔ `eframe`).
- اتصال اینترنت فقط برای بار اول، تا Cargo وابستگی‌ها را دریافت کند؛ اجرای برنامه و رمزگذاری بسته‌ها آفلاین است.
- روی Linux ممکن است بسته‌های توسعهٔ X11/Wayland سیستم لازم باشند. در Windows، برای target مربوط به MSVC، Visual Studio Build Tools با C++ workload لازم است.

## اجرا روی Windows

۱. Rust را از وب‌سایت رسمی نصب کنید: <https://rustup.rs/>

۲. فایل ZIP را Extract کنید و در PowerShell وارد پوشهٔ پروژه شوید:

```powershell
cd path\to\privora-rust-v03
rustc --version
cargo --version
cargo run --release -p privora-desktop
```

اولین اجرا زمان بیشتری می‌برد چون Cargo وابستگی‌ها را دانلود و کامپایل می‌کند.

## تست و بررسی کد

از ریشهٔ پروژه:

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```

برای اجرای اپ در حالت توسعه:

```powershell
cargo run -p privora-desktop
```

## ساخت خروجی دسکتاپ

### Windows

از خود Windows و Rust MSVC استفاده کنید:

```powershell
rustup show
cargo build --release -p privora-desktop
```

فایل اجرایی معمولاً اینجاست:

```text
target\release\privora-desktop.exe
```

### ساخت Setup.exe برای نصب کاربران

برای ساخت installer، راهنمای [Windows Installer](docs/WINDOWS_INSTALLER.md) و اسکریپت `packaging/windows/Privora.iss` را ببین. ساده‌ترین روش این است که ابتدا Inno Setup 6 را نصب کنی و سپس از ریشهٔ پروژه اجرا کنی:

```powershell
.\scripts\build-windows.ps1 -Installer -Test
```

فایل نصب در `dist\Privora-Setup-0.3.0-x64.exe` ساخته می‌شود. این مسیر Setup.exe می‌سازد، نه MSI. توضیحات کامل در راهنمای Windows آمده است.

### Linux

```bash
cargo build --release -p privora-desktop
./target/release/privora-desktop
```

### macOS

```bash
cargo build --release -p privora-desktop
open target/release/privora-desktop
```

برای انتشار روی سیستم‌عامل دیگری، خروجی باینری را روی همان سیستم‌عامل یا در محیط Cross-compile مناسب آن بسازید. از یک `.exe` نمی‌توان APK یا فایل اجرایی macOS تولید کرد؛ هر هدف ساخت، toolchain خودش را می‌خواهد.

## فرایند ساخت و بازکردن بسته

۱. وارد **Create Package** شوید و یک یا چند فایل را انتخاب کنید.
۲. نام بسته، برچسب گیرنده (صرفاً توضیحی)، یادداشت محرمانه (اختیاری)، تاریخ شروع اعتبار، تاریخ انقضا و سقف بازکردن محلی را تعیین کنید.
۳. روی **Create Encrypted .vora** بزنید و مسیر ذخیره را مشخص کنید.
۴. برنامه فایل رمزگذاری‌شدهٔ `.vora` را می‌سازد و کد دسترسی را در صفحه نمایش می‌دهد. اگر به یادداشت مجوز نیاز داری، آن را با دکمهٔ جداگانه و در مسیر دلخواه ذخیره کن.
۵. فایل `.vora` را به گیرنده بدهید و کد دسترسی را از کانالی جداگانه به او برسانید.
۶. گیرنده در **Files / Unlock** فایل `.vora` را انتخاب می‌کند، کد را وارد می‌کند و پوشهٔ استخراج را مشخص می‌کند.
۷. در صورت معتبر بودن کد و سیاست محلی، فایل‌ها در یک زیرپوشهٔ جدید استخراج می‌شوند.

**اگر کسی هم فایل `.vora` و هم کد دسترسی را داشته باشد، می‌تواند محتوا را رمزگشایی کند.** فایل `.license.txt` را کنار بسته در یک کانال عمومی منتشر نکنید.

## محدودیت‌های مهم نسخهٔ آفلاین

این نسخه برای اثبات فرمت و گردش‌کار رمزگذاری است، نه DRM غیرقابل‌شکست:

- شمارندهٔ سقف بازکردن در فایل وضعیت محلی برنامه نگهداری می‌شود. حذف داده‌های برنامه یا کپی بسته به دستگاه دیگر می‌تواند شمارنده را دور بزند. بدون یک مرجع آنلاین یا فرایند فعال‌سازی جداگانه، شمارش جهانی اشخاص/دستگاه‌ها قابل تضمین نیست.
- تاریخ انقضا با ساعت محلی دستگاه مقایسه می‌شود و تغییر ساعت سیستم می‌تواند بررسی زمان را دور بزند.
- فایل‌های استخراج‌شده plaintext هستند و پس از استخراج، برنامه نمی‌تواند جلوی کپی، ضبط صفحه یا پخش بیرونی آن‌ها را بگیرد.
- برای ساخت و بازکردن بسته، این آلفا موقتاً یک ZIP بدون رمز روی دیسک موقت سیستم می‌سازد؛ پاک‌سازی عادی تضمین‌کنندهٔ پاک‌شدن امن فیزیکی روی SSD یا فایل‌سیستم‌های ژورنال‌دار نیست. برای داده‌های بسیار حساس یا دستگاه مشترک مناسب نیست.
- تاریخچهٔ محلی سازنده شامل بخشی از metadata بسته‌ها و برچسب گیرنده است و به‌صورت JSON در پروفایل کاربر ذخیره می‌شود؛ متن یادداشت محرمانه از این تاریخچه حذف می‌شود. این با هدر رمزگذاری‌شدهٔ خود فایل `.vora` فرق دارد.
- نسخهٔ فعلی یک کد دسترسی برای هر بسته دارد؛ صدور چند مجوز مستقل و لغو از راه دور هنوز پیاده نشده است.
- فراموشی کد دسترسی به معنی از دست‌دادن امکان رمزگشایی است؛ هیچ backdoor یا بازیابی ابری وجود ندارد.
- استفادهٔ عمومی برای اسناد حساس/تجاری را تا بازبینی مستقل پروتکل و کد توصیه نمی‌کنیم.

## محدودهٔ پلتفرم

این ZIP شامل کلاینت **Native دسکتاپ** برای Windows، Linux و macOS است و منطق اصلی `.vora` در crate مستقل Rust قرار دارد. این نسخه هنوز APK واقعی نمی‌سازد: فایل‌انتخاب فعلی `rfd` از Android پشتیبانی نمی‌کند و برای Android به Activity/Gradle و اتصال به Android Storage Access Framework نیاز داریم. مراحل نصب ابزارها و گام‌های باقی‌مانده در [Android Build](docs/ANDROID_BUILD.md) مشخص شده‌اند. از اضافه‌کردن صرفِ target به این پروژه، APK قابل‌استفاده به دست نمی‌آید.

## راهنماهای نسخهٔ 0.3

- [راهنمای نصب‌کنندهٔ Windows](docs/WINDOWS_INSTALLER.md)
- [وضعیت و پیش‌نیازهای Android](docs/ANDROID_BUILD.md)
- [جهت‌گیری طراحی UI/UX](docs/DESIGN_GUIDE.md)
- [چک‌لیست پیش از انتشار](docs/RELEASE_CHECKLIST.md)

## ساختار ریپو

```text
privora-rust-v03/
├── Cargo.toml
├── README.md
├── AGENTS.md
├── crates/
│   └── privora-core/
│       ├── Cargo.toml
│       └── src/lib.rs       # .vora v1/v2, Argon2id, HKDF, AES-GCM, policy
├── apps/
│   └── privora-desktop/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs      # native entry point
│           ├── app.rs       # egui screens and worker threads
│           └── state.rs     # minimal local package/unlock history
├── packaging/windows/       # Inno Setup script
├── scripts/                 # Windows build helper
└── docs/                    # protocol, security, installers, Android, design
```

## منابع فنی

<<<<<<< HEAD
- `eframe`: <https://docs.rs/eframe/latest/eframe/>
- `aes-gcm`: <https://docs.rs/aes-gcm/latest/aes_gcm/>
- `argon2`: <https://docs.rs/argon2/latest/argon2/>
- `rfd`: <https://docs.rs/rfd/latest/rfd/>
- `zip`: <https://docs.rs/zip/latest/zip/>
=======
Privora is **source-available, not OSI-approved open-source software**. The custom [Privora Personal Learning and Non-Commercial License](LICENSE) allows private personal use, educational study, and non-commercial experimentation while restricting commercial use and redistribution. Read the complete terms before using the source or binaries.

If you need to prevent the public from viewing or forking the source, keep the repository private. GitHub's own Terms of Service govern public repository viewing and platform-hosted forks; a custom license cannot turn a public repository into a private one.

---

## ⭐ Support the project

If Privora helps you with your works, starring the repository is a simple way to support the project ⭐ .

---

<div align="center">

**PRIVORA** · Create · Encrypt · License · Share

</div>
>>>>>>> f13d45950d242710e80d887b396ca95e8288690e
