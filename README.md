# Simple SF

> 🌍 [English](#english) · [Français](#français) · [Deutsch](#deutsch) · [中文](#中文) · [日本語](#日本語) · [Español](#español) · [Português](#português) · [Русский](#русский) · [한국어](#한국어) · [العربية](#العربية) · [हिन्दी](#हिन्दी) · [Italiano](#italiano) · [Nederlands](#nederlands) · [Polski](#polski) · [Türkçe](#türkçe) · [Tiếng Việt](#tiếng-việt) · [Indonesia](#indonesia) · [ไทย](#ไทย) · [Čeština](#čeština) · [Svenska](#svenska) · [Dansk](#dansk) · [Suomi](#suomi) · [Norsk](#norsk) · [Română](#română) · [Magyar](#magyar) · [Ελληνικά](#ελληνικά) · [עברית](#עברית) · [Slovenčina](#slovenčina) · [Hrvatski](#hrvatski) · [Slovenščina](#slovenščina) · [Srpski](#srpski) · [Українська](#українська) · [Català](#català) · [Melayu](#melayu) · [Filipino](#filipino) · [বাংলা](#বাংলা) · [اردو](#اردو) · [Afrikaans](#afrikaans) · [Lietuvių](#lietuvių) · [Latviešu](#latviešu)

---

## English

**Simple SF** is a native macOS app that packages the entire [Software Factory](https://github.com/sylvain-legland/software-factory) platform inside a single `.app` — no server setup, no Docker, no external dependencies.

### Features
- **Jarvis** — streaming AI assistant with full context
- **Ideation** — AI teams brainstorming in parallel (3–5 agents, real personas)
- **Projects** — progress bar · start / pause / stop buttons · live spinner
- **Full SF in Advanced mode** — 133+ agents, 12 patterns, SAFe, A2A, RLM
- **8 LLM providers** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 languages** — full i18n, auto-detected from system locale
- **ZIP export** — package your project as a `.zip` with one click
- **Git push** — push directly to GitHub or GitLab (private or public)

### Requirements
- macOS 14 (Sonoma) or later
- Xcode 15+ (to build from source)

### Quick start
```bash
git clone https://github.com/sylvain-legland/simple-sf
cd simple-sf
./Scripts/embed_python.sh   # bundles Python 3.12 + SF backend (~5 min, once)
open Package.swift           # opens in Xcode → Build & Run
```

### Simple ↔ Advanced mode
Toggle in the toolbar. **Simple** shows 3 views (Projects, Jarvis, Ideation). **Advanced** unlocks the full SF platform: Portfolio, PI Board, ART, Backlog, Metrics, Live, Workflows, Agents.

### Architecture
```
SimpleSF.app
├── MacOS/SimpleSF              Swift binary
├── Frameworks/Python.framework Python 3.12 runtime (embedded)
└── Resources/
    ├── platform/               SF Python backend (FastAPI)
    ├── site-packages/          pip dependencies
    └── *.lproj/                40 language bundles
```

---

## Français

**Simple SF** est une application macOS native qui embarque l'intégralité de la plateforme [Software Factory](https://github.com/sylvain-legland/software-factory) dans un seul `.app` — pas de serveur à configurer, pas de Docker, aucune dépendance externe.

### Fonctionnalités
- **Jarvis** — assistant IA en streaming avec contexte complet
- **Idéation** — équipes IA qui brainstorment en parallèle (3–5 agents, vrais personas)
- **Projets** — barre de progression · boutons démarrer / pause / arrêt · spinner live
- **SF complète en mode Avancé** — 133+ agents, 12 patterns, SAFe, A2A, RLM
- **8 fournisseurs LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 langues** — i18n complète, détection automatique depuis la langue système
- **Export ZIP** — packager votre projet en `.zip` en un clic
- **Git push** — pousser directement sur GitHub ou GitLab (privé ou public)

### Prérequis
- macOS 14 (Sonoma) ou supérieur
- Xcode 15+ (pour compiler depuis les sources)

### Démarrage rapide
```bash
git clone https://github.com/sylvain-legland/simple-sf
cd simple-sf
./Scripts/embed_python.sh   # embarque Python 3.12 + backend SF (~5 min, une fois)
open Package.swift           # ouvre dans Xcode → Build & Run
```

---

## Deutsch

**Simple SF** ist eine native macOS-App, die die gesamte [Software Factory](https://github.com/sylvain-legland/software-factory)-Plattform in einer einzigen `.app` bündelt — kein Server-Setup, kein Docker, keine externen Abhängigkeiten.

### Funktionen
- **Jarvis** — Streaming-KI-Assistent mit vollem Kontext
- **Ideation** — KI-Teams, die parallel brainstormen (3–5 Agents, echte Personas)
- **Projekte** — Fortschrittsbalken · Start / Pause / Stopp · Live-Spinner
- **Vollständige SF im Erweiterten Modus** — 133+ Agents, 12 Muster, SAFe, A2A, RLM
- **8 LLM-Anbieter** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 Sprachen** — vollständige i18n, automatisch aus Systemsprache erkannt

### Schnellstart
```bash
git clone https://github.com/sylvain-legland/simple-sf
./Scripts/embed_python.sh && open Package.swift
```

---

## 中文

**Simple SF** 是一款原生 macOS 应用，将完整的 [Software Factory](https://github.com/sylvain-legland/software-factory) 平台打包进单个 `.app` — 无需配置服务器，无需 Docker，无外部依赖。

### 功能特点
- **Jarvis** — 具有完整上下文的流式 AI 助手
- **创意工坊** — AI 团队并行头脑风暴（3–5 个智能体，真实人物角色）
- **项目管理** — 进度条 · 开始 / 暂停 / 停止按钮 · 实时加载指示器
- **高级模式完整 SF** — 133+ 智能体、12 种模式、SAFe、A2A、RLM
- **8 个 LLM 提供商** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 种语言** — 完整国际化，自动检测系统语言

### 快速开始
```bash
git clone https://github.com/sylvain-legland/simple-sf
./Scripts/embed_python.sh && open Package.swift
```

---

## 日本語

**Simple SF** は、[Software Factory](https://github.com/sylvain-legland/software-factory) プラットフォーム全体を1つの `.app` にパッケージした ネイティブ macOS アプリです — サーバーのセットアップ不要、Docker不要、外部依存なし。

### 機能
- **Jarvis** — フルコンテキストのストリーミング AI アシスタント
- **アイデア出し** — AI チームが並行してブレインストーミング（3〜5 エージェント、リアルなペルソナ）
- **プロジェクト** — 進捗バー · 開始 / 一時停止 / 停止ボタン · ライブスピナー
- **高度モードの完全 SF** — 133以上のエージェント、12パターン、SAFe、A2A、RLM
- **8 LLM プロバイダー** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 言語** — 完全 i18n、システムロケールから自動検出

### クイックスタート
```bash
git clone https://github.com/sylvain-legland/simple-sf
./Scripts/embed_python.sh && open Package.swift
```

---

## Español

**Simple SF** es una aplicación nativa de macOS que empaqueta toda la plataforma [Software Factory](https://github.com/sylvain-legland/software-factory) en una sola `.app` — sin configuración de servidor, sin Docker, sin dependencias externas.

### Características
- **Jarvis** — asistente de IA en streaming con contexto completo
- **Ideación** — equipos de IA haciendo brainstorming en paralelo (3–5 agentes, personas reales)
- **Proyectos** — barra de progreso · botones iniciar / pausar / detener · spinner en vivo
- **SF completa en modo Avanzado** — 133+ agentes, 12 patrones, SAFe, A2A, RLM
- **8 proveedores LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 idiomas** — i18n completa, detección automática desde el idioma del sistema

---

## Português

**Simple SF** é um app nativo para macOS que empacota toda a plataforma [Software Factory](https://github.com/sylvain-legland/software-factory) em um único `.app` — sem configuração de servidor, sem Docker, sem dependências externas.

### Funcionalidades
- **Jarvis** — assistente de IA em streaming com contexto completo
- **Ideação** — equipes de IA fazendo brainstorming em paralelo (3–5 agentes, personas reais)
- **Projetos** — barra de progresso · botões iniciar / pausar / parar · spinner ao vivo
- **SF completa no modo Avançado** — 133+ agentes, 12 padrões, SAFe, A2A, RLM
- **8 provedores LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 idiomas** — i18n completa, detecção automática pelo idioma do sistema

---

## Русский

**Simple SF** — нативное приложение для macOS, которое упаковывает всю платформу [Software Factory](https://github.com/sylvain-legland/software-factory) в один `.app` — без настройки сервера, без Docker, без внешних зависимостей.

### Возможности
- **Jarvis** — потоковый ИИ-ассистент с полным контекстом
- **Идеация** — команды ИИ, проводящие мозговой штурм параллельно (3–5 агентов, реальные персоны)
- **Проекты** — прогресс-бар · кнопки старт / пауза / стоп · живой спиннер
- **Полная SF в расширенном режиме** — 133+ агентов, 12 паттернов, SAFe, A2A, RLM
- **8 провайдеров LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 языков** — полная i18n, автоопределение из системной локали

---

## 한국어

**Simple SF**는 전체 [Software Factory](https://github.com/sylvain-legland/software-factory) 플랫폼을 단일 `.app`으로 패키징한 네이티브 macOS 앱입니다 — 서버 설정 불필요, Docker 불필요, 외부 종속성 없음.

### 기능
- **Jarvis** — 전체 컨텍스트를 갖춘 스트리밍 AI 어시스턴트
- **아이디어 발상** — AI 팀이 병렬로 브레인스토밍 (3–5 에이전트, 실제 페르소나)
- **프로젝트** — 진행률 표시줄 · 시작 / 일시정지 / 정지 버튼 · 라이브 스피너
- **고급 모드 전체 SF** — 133+ 에이전트, 12 패턴, SAFe, A2A, RLM
- **8개 LLM 제공업체** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40개 언어** — 완전한 i18n, 시스템 로케일에서 자동 감지

---

## العربية

**Simple SF** هو تطبيق macOS أصلي يحزم منصة [Software Factory](https://github.com/sylvain-legland/software-factory) بالكامل في ملف `.app` واحد — بدون إعداد خادم، بدون Docker، بدون تبعيات خارجية.

### المميزات
- **Jarvis** — مساعد ذكاء اصطناعي متدفق مع سياق كامل
- **توليد الأفكار** — فرق الذكاء الاصطناعي تعصف ذهنياً بالتوازي (3–5 وكلاء، شخصيات حقيقية)
- **المشاريع** — شريط تقدم · أزرار البدء / الإيقاف المؤقت / الإيقاف · مؤشر دوار مباشر
- **SF كاملة في الوضع المتقدم** — 133+ وكيل، 12 نمطاً، SAFe، A2A، RLM
- **8 مزودي LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 لغة** — i18n كاملة، اكتشاف تلقائي من لغة النظام

---

## हिन्दी

**Simple SF** एक नेटिव macOS ऐप है जो पूरे [Software Factory](https://github.com/sylvain-legland/software-factory) प्लेटफ़ॉर्म को एक `.app` में पैकेज करती है — कोई सर्वर सेटअप नहीं, कोई Docker नहीं, कोई बाहरी निर्भरता नहीं।

### विशेषताएं
- **Jarvis** — पूर्ण संदर्भ के साथ स्ट्रीमिंग AI सहायक
- **विचार-मंथन** — AI टीमें समानांतर में ब्रेनस्टॉर्म करती हैं (3–5 एजेंट, वास्तविक पर्सोना)
- **प्रोजेक्ट** — प्रगति बार · शुरू / रोकें / बंद करें बटन · लाइव स्पिनर
- **उन्नत मोड में पूर्ण SF** — 133+ एजेंट, 12 पैटर्न, SAFe, A2A, RLM
- **8 LLM प्रदाता** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 भाषाएं** — पूर्ण i18n, सिस्टम लोकेल से स्वचालित पहचान

---

## Italiano

**Simple SF** è un'app nativa per macOS che racchiude l'intera piattaforma [Software Factory](https://github.com/sylvain-legland/software-factory) in un unico `.app` — nessuna configurazione server, nessun Docker, nessuna dipendenza esterna.

### Funzionalità
- **Jarvis** — assistente AI in streaming con contesto completo
- **Ideazione** — team AI che fanno brainstorming in parallelo (3–5 agenti, persone reali)
- **Progetti** — barra di avanzamento · pulsanti avvia / pausa / ferma · spinner live
- **SF completa in modalità Avanzata** — 133+ agenti, 12 pattern, SAFe, A2A, RLM
- **8 provider LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 lingue** — i18n completa, rilevamento automatico dalla lingua di sistema

---

## Nederlands

**Simple SF** is een native macOS-app die het volledige [Software Factory](https://github.com/sylvain-legland/software-factory)-platform verpakt in één `.app` — geen serverinstallatie, geen Docker, geen externe afhankelijkheden.

### Functies
- **Jarvis** — streaming AI-assistent met volledige context
- **Ideation** — AI-teams brainstormen parallel (3–5 agents, echte persona's)
- **Projecten** — voortgangsbalk · start / pauzeer / stop knoppen · live spinner
- **Volledige SF in Geavanceerde modus** — 133+ agents, 12 patronen, SAFe, A2A, RLM
- **8 LLM-aanbieders** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 talen** — volledige i18n, automatische detectie van systeemtaal

---

## Polski

**Simple SF** to natywna aplikacja macOS, która pakuje całą platformę [Software Factory](https://github.com/sylvain-legland/software-factory) w jeden plik `.app` — bez konfiguracji serwera, bez Dockera, bez zewnętrznych zależności.

### Funkcje
- **Jarvis** — strumieniowy asystent AI z pełnym kontekstem
- **Ideacja** — zespoły AI przeprowadzające burzę mózgów równolegle (3–5 agentów, prawdziwe persony)
- **Projekty** — pasek postępu · przyciski start / pauza / stop · live spinner
- **Pełna SF w trybie Zaawansowanym** — 133+ agentów, 12 wzorców, SAFe, A2A, RLM
- **8 dostawców LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 języków** — pełna i18n, automatyczne wykrywanie języka systemu

---

## Türkçe

**Simple SF**, [Software Factory](https://github.com/sylvain-legland/software-factory) platformunun tamamını tek bir `.app` içinde paketleyen yerel bir macOS uygulamasıdır — sunucu kurulumu yok, Docker yok, harici bağımlılık yok.

### Özellikler
- **Jarvis** — tam bağlamlı akış AI asistanı
- **Fikir Üretimi** — AI ekipleri paralel olarak beyin fırtınası yapıyor (3–5 ajan, gerçek kişilikler)
- **Projeler** — ilerleme çubuğu · başlat / duraklat / durdur düğmeleri · canlı döndürücü
- **Gelişmiş modda tam SF** — 133+ ajan, 12 desen, SAFe, A2A, RLM
- **8 LLM sağlayıcısı** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 dil** — tam i18n, sistem dilinden otomatik algılama

---

## Tiếng Việt

**Simple SF** là ứng dụng macOS gốc đóng gói toàn bộ nền tảng [Software Factory](https://github.com/sylvain-legland/software-factory) vào một `.app` duy nhất — không cần cài đặt máy chủ, không Docker, không phụ thuộc bên ngoài.

### Tính năng
- **Jarvis** — trợ lý AI phát trực tuyến với đầy đủ ngữ cảnh
- **Ý tưởng** — các nhóm AI cùng brainstorm song song (3–5 tác nhân, nhân vật thực)
- **Dự án** — thanh tiến trình · nút bắt đầu / tạm dừng / dừng · spinner trực tiếp
- **SF đầy đủ trong chế độ Nâng cao** — 133+ tác nhân, 12 mẫu, SAFe, A2A, RLM
- **8 nhà cung cấp LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 ngôn ngữ** — i18n đầy đủ, tự động phát hiện từ ngôn ngữ hệ thống

---

## Indonesia

**Simple SF** adalah aplikasi macOS asli yang mengemas seluruh platform [Software Factory](https://github.com/sylvain-legland/software-factory) ke dalam satu `.app` — tanpa pengaturan server, tanpa Docker, tanpa dependensi eksternal.

### Fitur
- **Jarvis** — asisten AI streaming dengan konteks penuh
- **Ideasi** — tim AI melakukan brainstorming secara paralel (3–5 agen, persona nyata)
- **Proyek** — bilah kemajuan · tombol mulai / jeda / berhenti · spinner langsung
- **SF lengkap dalam mode Lanjutan** — 133+ agen, 12 pola, SAFe, A2A, RLM
- **8 penyedia LLM** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 bahasa** — i18n lengkap, deteksi otomatis dari bahasa sistem

---

## ไทย

**Simple SF** คือแอป macOS แบบเนทีฟที่รวมแพลตฟอร์ม [Software Factory](https://github.com/sylvain-legland/software-factory) ทั้งหมดไว้ใน `.app` เดียว — ไม่ต้องตั้งค่าเซิร์ฟเวอร์ ไม่ต้องใช้ Docker ไม่มีการพึ่งพาภายนอก

### คุณสมบัติ
- **Jarvis** — ผู้ช่วย AI แบบสตรีมมิ่งพร้อมบริบทเต็มรูปแบบ
- **ระดมความคิด** — ทีม AI ระดมความคิดแบบขนาน (3–5 เอเจนต์ บุคลิกที่แท้จริง)
- **โครงการ** — แถบความคืบหน้า · ปุ่มเริ่ม / หยุดชั่วคราว / หยุด · สปินเนอร์สด
- **SF เต็มรูปแบบในโหมดขั้นสูง** — 133+ เอเจนต์ 12 รูปแบบ SAFe A2A RLM
- **ผู้ให้บริการ LLM 8 ราย** — OpenRouter · OpenAI · Anthropic · Gemini · Kimi · MiniMax · Qwen · GLM
- **40 ภาษา** — i18n เต็มรูปแบบ ตรวจจับอัตโนมัติจากภาษาของระบบ

---

## Čeština · Svenska · Dansk · Suomi · Norsk · Română · Magyar · Ελληνικά · עברית · Slovenčina · Hrvatski · Slovenščina · Srpski · Українська · Català · Melayu · Filipino · বাংলা · اردو · Afrikaans · Lietuvių · Latviešu

> Tyto jazyky jsou plně podporovány v aplikaci. / Dessa språk stöds fullt ut i appen. / Disse sprog understøttes fuldt ud i appen. / Nämä kielet ovat täysin tuettuja sovelluksessa.

**Simple SF** packages the entire Software Factory into a single macOS app with full support for 40 languages. Each language is auto-detected from your system locale and can be changed in Settings → Language.

---

## Screenshots

> Real data from a live SF instance

![Projects view with running missions](docs/screenshots/projects.png)
![Jarvis AI chat streaming](docs/screenshots/jarvis.png)
![Ideation — AI team debate](docs/screenshots/ideation.png)
![Advanced mode — Portfolio](docs/screenshots/portfolio.png)
![Onboarding — LLM providers](docs/screenshots/onboarding.png)

---

## License

MIT — © 2026 [Macaron Software](https://macaron-software.com)
