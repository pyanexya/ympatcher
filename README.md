<p align="center"><img src="assets/brand/ympatcher-mark.png" width="180" alt="ympatcher"></p>

# ympatcher

Локальный Rust-инструмент для преобразования APKS/APKM в единый APK, исправления упаковки native-библиотек и применения проверяемых патчей к Android-клиенту Яндекс Музыки. Независимый проект Pyanexya aka Pyanexy, не связанный с Яндексом или Discord.

Здесь публикуются исходники и сборки **самого инструмента**. APK клиента, закрытый Discord SDK, ключи подписи и выгрузки содержимого приложения сюда не входят. Входной файл предоставляет пользователь.

## APKS → APK и исправление `.so`

Нужны JDK 17+ (`java` и `keytool` в PATH) и [собранный ympatcher](https://github.com/pyanexya/ympatcher/releases) либо Rust toolchain для сборки:

```sh
cargo build --release --locked
# Windows: target/release/ympatcher.exe
```

Преобразовать подписанный APKS/APKM без применения новых патчей:

```sh
ympatcher convert music.apks -o music-fixed.apk
ympatcher convert music.apkm -o music-fixed.apk
```

Исправить упаковку существующего APK:

```sh
ympatcher convert music.apk -o music-fixed.apk
```

`convert` проверяет подписи входных файлов, общий сертификат, package и versionCode splits. APKEditor объединяет manifest, таблицы ресурсов и файлы. Затем ympatcher хранит все `.so` и `resources.arsc` без сжатия, выравнивает `.so` на 16 КБ, остальные несжатые entries на 4 байта, подписывает APK и повторно проверяет подпись и ZIP layout. Результат — APK приложения, который открывается обычным установщиком Android.

Поддерживается один base с уникальными `config.*` splits (ABI, density, language), включая папку `splits/`. Выход содержит только архитектуры и ресурсы, имеющиеся во входе. Dynamic features и архивы с несколькими наборами для разных устройств отклоняются. Выравнивание ZIP не исправляет ELF-библиотеку, собранную только для страниц 4 КБ; для полной поддержки устройств с 16 КБ может потребоваться пересборка самой библиотеки.

## Патчи клиента

```sh
# Проверенный исходный APK/APKS/APKM → пропатченный единый APK
ympatcher official.apkm -o music-patched.apk

# Сохранить split-формат
ympatcher official.apkm -o music-patched.apks

# Только выбранные патчи
ympatcher official.apk --patch about -o music-patched.apk
ympatcher --list-patches
```

Стандартный профиль добавляет «О приложении», Developer experiments с локальными overrides и совместимость Passport с изменённой подписью. `discord-rpc` включается отдельно и требует законно полученный Social SDK и локально собранный embedded AAR; [инструкция](presence/README.md). Presence по умолчанию выключен. Русский/английский интерфейс и светлая/тёмная тема поддерживаются.

Патчер проверяет package name, официальный сертификат, SHA-256, compatibility manifest и fingerprints патчей. `convert` не применяет патчи и не требует compatibility manifest, поэтому пригоден и для ранее пропатченного APKS. `--latest --channel stable|beta` — дополнительный сетевой сценарий через API проекта; работоспособность внешнего API не гарантируется, локальный ввод от него не зависит.

## Подпись и обновления

**Используйте один `--state-dir` для всех сборок, которые должны обновляться поверх друг друга.** В нём хранится приватный ключ. Не коммитьте и не публикуйте этот каталог. Первый запуск создаёт локальный ключ, если существующий не найден; новый ключ несовместим с подписью предыдущих установок.

```sh
ympatcher convert music.apks -o music-fixed.apk --state-dir /path/to/existing-state
```

После пересборки оригинальная подпись APK теряется. Переход с официальной подписи на собственную обычно требует удаления установленного клиента с его локальными данными. Сначала убедитесь, что нужные данные синхронизированы. `--install` в сценарии патчера проверяет сертификат и downgrade до установки через ADB. Команда `convert` только сохраняет файл.

## Проверки и ограничения

```sh
python scripts/check_publication.py
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all --locked
cargo build --release --locked
```

Регрессионные тесты проверяют упаковку, сохранность содержимого, небезопасные пути, повреждённые входы и прежнее поведение патчера. Тесты discovery с реальной сетью требуют `YM_DATAMINER_LIVE_TESTS=1`. Android-установка, воспроизведение, авторизация и Discord Presence требуют проверки на устройстве; успешная сборка сама по себе не подтверждает их работу.

Разработка ведётся в `main`; релизные теги `vX.Y.Z`, предварительные `vX.Y.Z-beta.N`/`-rc.N`/`-dev.N`. [Аудит](docs/AUDIT.md), [Actions](docs/ACTIONS.md), [Changelog](CHANGELOG.md), [вклад в проект](CONTRIBUTING.md).

## Правовой статус

MIT распространяется на собственный код проекта. APK, контент, названия и товарные знаки принадлежат соответствующим правообладателям. Пользователь должен иметь права на использование и изменение своего входного файла; модификация клиента может противоречить условиям сервиса. Дисклеймер не предоставляет права на чужой код и не гарантирует отсутствие претензий или блокировок платформой.

[License](LICENSE) · [Third-party notices](THIRD_PARTY_NOTICES.md) · [Security](SECURITY.md) · [Privacy](PRIVACY.md) · [Disclaimer](DISCLAIMER.md) · [Условия RU](TERMS.ru.md) · [Terms EN](TERMS.en.md).
