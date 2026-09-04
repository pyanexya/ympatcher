# Changelog

## v0.5.5 — dev

Все возможности v0.5.0 на latest stable Яндекс Музыке 2026.08.4 #162.1gpr. Отдельный канал разработки; релиз помечается prerelease.

## v0.5.0 — stable

- APKS/APKM преобразуются в настоящий APK с объединением manifest, ресурсов и ABI splits.
- Команда `convert` исправляет уже подписанные APK/APKS без повторного применения патчей.
- Native .so упаковываются без сжатия и выравниваются на 16 КБ до подписи; итоговая подпись и ZIP layout проверяются.
- About, 155 Developer experiments, Passport compatibility и опциональный Discord Presence.
- Проверка сертификатов, hashes и fingerprints, безопасный импорт и сохранение прежнего результата при ошибке.
- Подпись одним постоянным локальным ключом; проверка обновления через ADB.
- CLI dataminer, локальный ввод и latest API сохранены.
- Зависимости обновлены; cargo audit без предупреждений.
- GitHub: stable/dev, проверка версий, CI Linux/Windows, отдельные релизы и ручная сборка клиента.
- Удалены устаревший APK-wrapper, дублирующие документы и неиспользуемые скриншоты.

Проверки: 43 unit-теста, fmt/clippy, Windows release build, Android embedded AAR для четырёх ABI, сборка latest APK, подпись и zipalign. GitHub CI также проходит на Linux и Windows. Проверка на физическом устройстве ещё требуется.
