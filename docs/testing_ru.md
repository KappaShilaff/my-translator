# My Translator: запуск, ключи, проверка

Актуально для локального checkout репозитория.
Проверено на Linux с PipeWire/PulseAudio.

## Короткий вывод

Проект изначально был рассчитан на macOS 13+ и Windows 10/11. В этом checkout добавлен Linux backend для system audio и microphone через PipeWire/PulseAudio (`pw-record` + `pactl`).

Проверенный базовый набор команд:

```bash
npm install
npm run tauri -- --version
cargo check
npm run tauri -- dev
```

Результат:

- `npm install` прошел успешно, уязвимостей npm не найдено.
- Tauri CLI установлен локально: `tauri-cli 2.10.1`.
- `cargo check` проходит.
- Linux system audio и microphone backend добавлены, но для сборки и запуска всё равно нужны системные Tauri/WebKit dev-пакеты ниже.

Для реального теста на Linux нужны PipeWire/Pulse sources. `pw-record` должен открывать default speaker monitor для System Audio и default input source для Microphone.

## Как запускать из исходников

Установка зависимостей:

```bash
cd my-translator
npm install
```

Dev-запуск:

```bash
npm run tauri -- dev
```

Production build:

```bash
npm run tauri -- build
```

Версия локального проекта: `0.7.2`.
Последний проверенный GitHub release на 2026-06-01: `v0.7.2`, опубликован 2026-05-26.

## Быстрый путь без сборки

Для macOS/Windows проще скачать готовый release:

- macOS Apple Silicon: `MyTranslator_0.7.2_aarch64.dmg`
- macOS Intel: `MyTranslator_0.7.2_x64.dmg`
- Windows x64: `MyTranslator_0.7.2_x64-setup.exe`
- Windows MSI: `MyTranslator_0.7.2_x64_en-US.msi`

Release page: https://github.com/phuc-nt/my-translator/releases/latest

## Где брать ключи

Минимально нужен один engine key. Рекомендованный дефолт для обычного теста - Soniox.

### Soniox

Для чего: основной режим real-time speech-to-text + translation. Поддерживает source transcript, dual panel, one-way/two-way.

Где брать:

1. Открыть https://console.soniox.com
2. Создать аккаунт.
3. В Billing добавить payment method / funds.
4. В API Keys создать новый ключ.
5. Вставить ключ в app: Settings -> Translation -> Soniox API key.
6. Нажать Test рядом с ключом.

Ожидаемый формат по документации проекта: `soniox_...`.
Ориентировочная цена из README/docs: около `$0.12/hour` processed audio.

### OpenAI Realtime

Для чего: premium realtime engine, text + optional/native translated voice. В приложении используется `gpt-realtime-translate`.

Где брать:

1. Открыть https://platform.openai.com
2. Добавить billing/credits.
3. Открыть API keys: https://platform.openai.com/api-keys
4. Create new secret key.
5. Вставить ключ в app: Settings -> Translation -> OpenAI API key.
6. Нажать Test рядом с ключом.

Ожидаемый формат: `sk-...`.
Ограничения по app docs: дороже Soniox, около `$4/hour`; two-way mode недоступен в OpenAI mode.

### Qwen LiveTranslate Flash

Для чего: text-only realtime translation через Alibaba DashScope, free preview на момент документации проекта.

Где брать:

1. Открыть https://bailian.console.alibabacloud.com
2. Важно: выбрать регион Singapore до создания workspace/API key.
3. Активировать Model Studio / DashScope, если попросит.
4. API Keys -> Create API Key.
5. Вставить ключ в app: Settings -> Translation -> Qwen API key.

Критично: ключи из не-Singapore регионов дают `WebSocket error`, потому что app ходит в international endpoint.
Для Qwen надо явно выбрать source language; `Auto-detect` не использовать.

### TTS: Edge

Для чего: озвучка переводов поверх Soniox/Local.

Ключ не нужен. В Settings -> TTS выбрать Edge TTS, voice и speed, затем включить TTS кнопкой в основном окне.

### TTS: Google Cloud Text-to-Speech

Для чего: более качественная озвучка, особенно Vietnamese Chirp 3 HD.

Где брать:

1. Открыть https://console.cloud.google.com
2. Создать проект.
3. Enable API: Cloud Text-to-Speech API.
4. Credentials -> Create Credentials -> API Key.
5. Рекомендуется Restrict key -> Cloud Text-to-Speech API only.
6. Вставить ключ в app: Settings -> TTS -> Google Chirp 3 HD.

Ожидаемый формат: `AIza...`.

### TTS: ElevenLabs

Для чего: premium TTS / voice cloning.

Где брать:

1. Открыть https://elevenlabs.io
2. Создать аккаунт и выбрать подходящий paid/free план.
3. Profile / Developers -> API Keys.
4. Create API Key.
5. Вставить ключ в app: Settings -> TTS -> ElevenLabs.

## Что выбрать для первого теста

Самый практичный первый тест:

1. Engine: Soniox.
2. Audio source: Microphone, если тестируешь голосом; System Audio, если тестируешь YouTube/Zoom.
3. Translation type: One-way.
4. Source language: Auto или конкретный язык.
5. Target language: нужный язык, например `ru`, `en`, `vi`.
6. TTS: Off сначала. После проверки текста включить Edge TTS.

Если нужен самый дешевый текстовый тест: Qwen, но только с Singapore key и явным source language.
Если нужен лучший realtime voice output: OpenAI Realtime, но контролировать стоимость.

## Где хранятся настройки

Ключи сохраняются локально в settings JSON, не в `.env`.
Код строит путь как `dirs::config_dir()/com.personal.translator/settings.json`.

Ожидаемые пути:

- Linux: `~/.config/com.personal.translator/settings.json`
- macOS: `~/Library/Application Support/com.personal.translator/settings.json`
- Windows: `%APPDATA%\com.personal.translator\settings.json`

Файл появляется после первого Save в Settings.

Не коммитить этот файл и не вставлять реальные ключи в репозиторий.

## Checklist для ручного теста

1. Открыть приложение.
2. Settings -> вставить выбранный key.
3. Нажать Test рядом с key; должен быть зеленый статус.
4. Выбрать engine и языки.
5. Save & Close.
6. Нажать Start или `Cmd/Ctrl+Enter`.
7. Подать аудио:
   - Microphone: сказать 1-2 фразы.
   - System Audio: запустить YouTube/Zoom/podcast.
8. Проверить, что появляются source/translation строки.
9. Включить Edge TTS и проверить озвучку.
10. Открыть transcript через кнопку clipboard/list в UI и проверить, что session сохраняется.

## Частые Linux build blockers

Текущая ошибка:

```text
failed to run custom build command for `alsa-sys v0.3.1`
The system library `alsa` required by crate `alsa-sys` was not found.
The file `alsa.pc` needs to be installed
```

Пакеты, которые уже точно отсутствуют:

```bash
pkg-config --modversion alsa
pkg-config --modversion webkit2gtk-4.1
```

Если dev-пакеты не установлены, эти команды возвращают `Package ... not found`.

Для Linux build обычно нужны Tauri/WebKit/audio dependencies. Например, для dnf-based дистрибутивов это выглядит так. `alsa-lib-devel` больше не обязателен, потому что Linux audio backend использует `pw-record`, а не `cpal`:

```bash
sudo dnf install \
  alsa-lib-devel \
  webkit2gtk4.1-devel \
  openssl-devel \
  gtk3-devel \
  librsvg2-devel \
  libayatana-appindicator-gtk3-devel \
  pkgconf-pkg-config \
  pipewire-utils \
  pulseaudio-utils
```

Linux backend выбирает monitor source так:

1. `pactl get-default-sink`
2. `<default-sink>.monitor`
3. `pactl list sources` -> `object.serial` для этого monitor source
4. `pw-record --target <serial> --rate 16000 --channels 2 --format s16 --raw -`
5. Rust backend downmix stereo PCM в mono `s16le 16kHz`

Почему по serial: на некоторых headset-устройствах `pw-record --target <name>.monitor --channels 1` может подключиться к `capture_MONO`, то есть к микрофону. Target по `object.serial` + stereo capture подключается к `monitor_FL/monitor_FR`.
