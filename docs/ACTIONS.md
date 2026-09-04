# GitHub Actions

`Rust CI` проверяет публикационный состав, форматирование, clippy, unit-тесты и release build на Linux и Windows. Артефакты содержат только executable ympatcher.

`Release ympatcher` запускается по тегам `v*.*.*`, проверяет принадлежность commit ветке `main`, собирает сам инструмент для Linux/Windows и публикует его с SHA-256. Право `contents: write` есть только у publish job. Теги не создаются автоматически.

Автоматическая загрузка, модификация и публикация клиента Яндекс Музыки, выгрузок из APK и отправка dataminer-отчётов в Discord удалены из workflows. Для локальной обработки используйте команды из README. CI не требует signing keys, SDK URL, source APK URL, Google credentials или Discord webhook.

Не добавляйте APK/APKS/APKM, SDK AAR, keystore или каталог state в Releases и Actions artifacts. `scripts/check_publication.py` проверяет tracked files и известные шаблоны секретов, но не заменяет проверку происхождения и лицензий материалов.
