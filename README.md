<p align="center"><img src="assets/brand/ympatcher-mark.png" width="160" alt="ympatcher"></p>

# ympatcher

Патчер Яндекс Музыки на Rust: APKS/APKM → полноценный APK, исправление упаковки .so, Developer experiments, About и Discord Rich Presence.

## Версии

| Ветка | Релиз | Основа |
|---|---|---|
| [stable](https://github.com/pyanexya/ympatcher/tree/stable) | [v0.5.0](https://github.com/pyanexya/ympatcher/releases/tag/v0.5.0) | production |
| [dev](https://github.com/pyanexya/ympatcher/tree/dev) | [v0.5.5](https://github.com/pyanexya/ympatcher/releases/tag/v0.5.5), prerelease | разработка stable/beta fingerprints |

Ветки имеют одинаковый набор возможностей. `dev` предназначена для разработки, `stable` — для стабильных выпусков. Канал исходного клиента (`--channel stable|beta`) выбирается отдельно. Версия и канал патчера хранятся в `release.json`; CI проверяет их соответствие Cargo.toml и тегу.

## Запуск

Нужен JDK 17+ с `java` и `keytool` в PATH. Скачайте executable из [Releases](https://github.com/pyanexya/ympatcher/releases). Сборка из исходников: Rust 1.88+ и `cargo build --release --locked`; Windows executable — `target/release/ympatcher.exe`.

```sh
# Уже подписанный APKS/APKM → единый APK, без повторного применения патчей
ympatcher convert music.apks -o music.apk

# Исправить упаковку существующего APK
ympatcher convert input.apk -o fixed.apk

# Пропатчить проверенный standalone APK из production API
ympatcher --latest --channel stable --abi arm64-v8a -o music.apk
ympatcher --latest --channel beta --abi arm64-v8a -o music-beta.apk

# ABI определится через ADB; без устройства используется arm64-v8a
ympatcher --latest --channel stable --install

# Локальный источник; выход .apks сохраняет splits
ympatcher official.apkm -o music-patched.apk
ympatcher official.apkm -o music-patched.apks
ympatcher --list-patches
```

`--latest` получает metadata из `https://releases.pyanexy.cc/v1/releases/{stable|beta}/latest` и выбирает только явно заданный ABI. API уже объединил Google Play splits в standalone APK, поэтому такой artifact намеренно unsigned. Патчер принимает его только после проверки HTTPS origin, package/channel/version, provenance официального сертификата, размера и SHA-256, затем повторно сверяет package и версию после decode, применяет патчи и подписывает финальный APK постоянным ключом.

`convert` объединяет manifest, ресурсы и ABI splits для локальных `.apks`/`.apkm`. Все `.so` хранятся без сжатия с выравниванием 16 КБ, `resources.arsc` — без сжатия и с выравниванием 4 байта. Подпись и упаковка проверяются после сборки. Поддерживается один base с config splits; отсутствующие в источнике ABI или ресурсы не создаются. ZIP alignment не заменяет ELF alignment самих библиотек.

Стандартный локальный профиль: About, эксперименты с поиском/overrides и совместимость Passport с подписью патчера. `--patch ID` выбирает патчи. Выпускаемые проектом клиентские APK обязательно содержат `discord-rpc`; Presence при этом выключен по умолчанию и включается только пользователем в настройках. Обычная сборка самого CLI не требует закрытого Discord SDK, а ручная инъекция описана в [presence/README.md](presence/README.md).

## Подпись и установка

Сохраняйте один `--state-dir` для совместимых обновлений — там находится ваш приватный ключ. Если ключа нет, патчер создаёт новый; повреждённое или несогласованное состояние приводит к ошибке, а не к тихой замене ключа. APK с новым сертификатом нельзя установить поверх официального приложения или другой сборки с отличающимся сертификатом без удаления старой установки и её локальных данных.

`--install` проверяет подпись и versionCode через ADB до установки. `convert` только сохраняет файл. APK не загружается куда-либо при локальной обработке; сеть нужна для загрузки инструментов и опционального `--latest`.

## Actions

- **Rust CI**: Linux/Windows, проверка версий, fmt, clippy, тесты и сборка.
- **Release ympatcher**: запуск по тегу или вручную с выбором stable/dev. Публикует executable и SHA-256; dev помечается prerelease и не заменяет stable Latest.
- **Build latest client**: ручная сборка standalone APK для выбранных `stable|beta` и `arm64-v8a|armeabi-v7a`. Discord Presence входит обязательно, выключен до opt-in пользователя. Результат хранится в защищённом Actions artifact 3 дня и не публикуется как открытый GitHub Release.

Для клиентской сборки Actions обязательны `SIGNING_P12_BASE64`, `SIGNING_JSON_BASE64` и `DISCORD_SOCIAL_SDK_URL`. Ключи, пароли и SDK не коммитятся. Граница между публичным upstream API и будущей авторизованной выдачей патченных APK описана в [docs/distribution.md](docs/distribution.md).

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

Mock-тесты production API не требуют сети. Дополнительная live-проверка metadata запускается только с `YMPATCHER_LIVE_TESTS=1`.

Проверены сборка latest APK, 24 native-библиотеки, подпись, zipalign и unit-тесты. Установка, запуск, воспроизведение и Presence требуют проверки на устройстве — подключённого устройства при подготовке выпуска не было.

[Изменения](CHANGELOG.md) · [Security](SECURITY.md) · [MIT](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md). Проект не связан с Яндексом или Discord; их код, SDK, контент и товарные знаки не входят в лицензию MIT на собственный код патчера.

Оформление Discord-сервера и GitHub webhooks воспроизводимо настраиваются безопасным скриптом из [docs/discord-server.md](docs/discord-server.md). Секреты передаются только через переменные окружения и никогда не сохраняются в репозитории.
