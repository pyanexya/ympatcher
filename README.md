<p align="center"><img src="assets/brand/ympatcher-mark.png" width="160" alt="ympatcher"></p>

# ympatcher

Патчер Яндекс Музыки на Rust: APKS/APKM → полноценный APK, исправление упаковки .so, Developer experiments, About и Discord Rich Presence.

## Версии

| Ветка | Релиз | Основа |
|---|---|---|
| [stable](https://github.com/pyanexya/ympatcher/tree/stable) | [v0.5.0](https://github.com/pyanexya/ympatcher/releases/tag/v0.5.0) | latest stable: 2026.08.4 #162.1gpr |
| [dev](https://github.com/pyanexya/ympatcher/tree/dev) | [v0.5.5](https://github.com/pyanexya/ympatcher/releases/tag/v0.5.5), prerelease | та же свежая 2026.08.4 |

Ветки имеют одинаковый набор возможностей. `dev` предназначена для разработки, `stable` — для стабильных выпусков. Канал исходного клиента (`--channel stable|beta`) выбирается отдельно. Версия и канал патчера хранятся в `release.json`; CI проверяет их соответствие Cargo.toml и тегу.

## Запуск

Нужен JDK 17+ с `java` и `keytool` в PATH. Скачайте executable из [Releases](https://github.com/pyanexya/ympatcher/releases). Сборка из исходников: Rust 1.88+ и `cargo build --release --locked`; Windows executable — `target/release/ympatcher.exe`.

```sh
# Уже подписанный APKS/APKM → единый APK, без повторного применения патчей
ympatcher convert music.apks -o music.apk

# Исправить упаковку существующего APK
ympatcher convert input.apk -o fixed.apk

# Пропатчить свежую версию из API проекта
ympatcher --latest --channel stable -o music-patched.apk

# Локальный источник; выход .apks сохраняет splits
ympatcher official.apkm -o music-patched.apk
ympatcher official.apkm -o music-patched.apks
ympatcher --list-patches
```

`convert` объединяет manifest, ресурсы и ABI splits. Все .so хранятся без сжатия с выравниванием 16 КБ, resources.arsc — без сжатия и с выравниванием 4 байта. Подпись и упаковка проверяются после сборки. Поддерживается один base с config splits; отсутствующие в источнике ABI или ресурсы не создаются. ZIP alignment не заменяет ELF alignment самих библиотек.

Стандартный профиль: About, эксперименты с поиском/overrides и совместимость Passport с подписью патчера. `--patch ID` выбирает патчи. `discord-rpc` добавляется отдельно: [сборка Presence](presence/README.md). Presence выключен до включения пользователем; при включении выбранные сведения о треке передаются Discord.

## Подпись и установка

Сохраняйте один `--state-dir` для совместимых обновлений — там находится ваш приватный ключ. Если ключа нет, патчер создаёт новый. APK с новым сертификатом нельзя установить поверх приложения с другим сертификатом без удаления старой установки и её локальных данных.

`--install` проверяет подпись и versionCode через ADB до установки. `convert` только сохраняет файл. APK не загружается куда-либо при локальной обработке; сеть нужна для загрузки инструментов и опционального `--latest`.

## Actions

- **Rust CI**: Linux/Windows, проверка версий, fmt, clippy, тесты и сборка.
- **Release ympatcher**: запуск по тегу или вручную с выбором stable/dev. Публикует executable и SHA-256; dev помечается prerelease и не заменяет stable Latest.
- **Build latest client**: ручная сборка latest в APK/APKS с опциональным Presence. Результат хранится в Actions artifacts 3 дня.

Для постоянной подписи в Actions задайте `SIGNING_P12_BASE64` и `SIGNING_JSON_BASE64`. Без них создаётся новый ключ на каждый запуск. Для Presence нужен `DISCORD_SOCIAL_SDK_URL` с вашим SDK. Ключи и SDK не коммитятся.

## Dataminer и разработка

```sh
ympatcher dataminer latest --json
ympatcher official.apk --datamine report.json --datamine-markdown report.md
python scripts/check_publication.py
python scripts/check_release.py
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all --locked
```

Google Play discovery принимает `YM_GOOGLE_PLAY_AUTH_TOKEN`, `YM_GOOGLE_PLAY_GSF_ID` и опционально `YM_GOOGLE_PLAY_DEVICE_CONFIG_TOKEN`. Без сессии доступны независимые APKMirror/RuStore providers. Сетевые тесты включаются через `YM_DATAMINER_LIVE_TESTS=1`.

Проверены сборка latest APK, 24 native-библиотеки, подпись, zipalign и unit-тесты. Установка, запуск, воспроизведение и Presence требуют проверки на устройстве — подключённого устройства при подготовке выпуска не было.

[Изменения](CHANGELOG.md) · [Security](SECURITY.md) · [MIT](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md). Проект не связан с Яндексом или Discord; их код, SDK, контент и товарные знаки не входят в лицензию MIT на собственный код патчера.
