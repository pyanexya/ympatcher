## 0.6.0 — 2026-09-05

- Real APKS/APKM-to-APK merge and a separate `convert` command.
- Stored native libraries, 16 KiB ZIP alignment and verification after signing.
- Safer imports/download paths and atomic output replacement.
- Opt-in Presence with guarded native calls; preserved existing signing identity.
- Source/tool-only publication, Linux/Windows CI, dependency and secret-pattern audit.
- See `docs/AUDIT.md` for validation evidence and device-testing limitations.

# Changelog

Все заметные изменения проекта описываются в этом файле.

## 0.5.0 — 2026-09-03

- Latest Stable/Latest Beta переведены на `ympatcher.pyanexy.cc/v1/apks/latest`;
- добавлены retry, отмена, прогресс и обязательная проверка размера/SHA-256;
- реализована безопасная распаковка APKM, переподпись base и всех splits и выпуск устанавливаемого APKS;
- добавлена поддержка `2026.08.4 #162.1gpr`: новые устойчивые fingerprints About, Experiments и Passport;
- каталог содержит 155 актуальных мобильных экспериментов с реальными локальными overrides;
- Info показывает версию источника, канал, ABI/minSdk, патчи, дату сборки и SHA источника;
- встроен Discord Rich Presence через прямой MediaSession hook и официальный Social SDK;
- добавлен one-file Android installer APK: внутри хранится APKS и на телефоне выбираются совместимые splits;
- GitHub Actions больше не требует `SOURCE_APK_URL`: latest берётся из API, release обновляется автоматически;
- добавлены тесты API, HTTP-ошибок, unavailable release, hash/size и безопасной APKM-распаковки.

## 0.4.0 — 2026-09-03

- добавлен reusable upstream release dataminer: Google Play protobuf metadata, APKMirror и RuStore;
- phone/Wear OS и Google Play/RuStore streams разделены, есть resolver confidence и provider diagnostics;
- добавлены JSON schema v1, CLI `dataminer latest`, общий HTTP client, cancellation, retry и memory cache;
- добавлены ветки `stable`/`dev` и явные Android release channels;
- compatibility manifest разделён на stable и dev, схема расширена до v2;
- RuStore удалён как скрытый источник, добавлены provider abstraction и безопасный APK/APKS import;
- Google Play ограничения зафиксированы в ADR, неподдерживаемая загрузка завершается явной ошибкой;
- original, display и technical versions разделены; установка заранее блокирует downgrade;
- dataminer переведён на схему v2 с evidence, confidence и детерминированными hashes;
- smali scanner распараллелен и проверен на реальном `2026.08.3 #161rur` (52 914 classes, 5 132 findings);
- добавлены Android manifest/resource/smali scanners и JSON, Markdown, Discord dry-run reporters;
- Actions разделяют stable/dev reports, дедуплицируют diff и не публикуют source/patched APK;
- официальный Discord Social SDK companion prototype проверен на Android, подготовлен experimental embedded module;
- добавлены оригинальные RU/EN terms, privacy и disclaimer drafts.

## 0.3.0 — 2026-09-02

- добавлена поддержка Яндекс Музыки `2026.08.3 #161rur`;
- сведения о патче вынесены в локализованную панель;
- добавлены каталог патчей, version spoof и compatibility guard;
- добавлены брендовый знак и Android dataminer v1.

## 0.2.0 — 2026-08-27

- ребрендинг в `ympatcher`;
- добавлены Material Symbols `info` и `science`;
- русский/английский UI, светлая/тёмная тема;
- поиск, `Force all` и `Reset to default`;
- сохранена цепочка постоянной подписи.

## 0.1.1 — 2026-08-27

- первая версия патчера;
- сведения о патче и поиск экспериментов;
- постоянная подпись и безопасное обновление.
