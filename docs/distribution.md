# Выдача патченных APK

`releases.pyanexy.cc` — публичный источник только проверенных upstream artifacts. Он не является точкой выдачи патченных сборок и не принимает пользовательские credentials.

## Текущий build boundary

`Build latest client` получает immutable standalone APK из Releases API, проверяет provenance, патчит его, встраивает Discord Presence, подписывает постоянным ключом и публикует только короткоживущий GitHub Actions artifact. Workflow требует repository authentication, signing secrets и Discord SDK secret. Патченный APK не попадает в открытый GitHub Release.

Результат producer-части состоит из:

- подписанного `.apk`;
- соседнего `.apk.sha256`;
- `assets/danger-patch.json` внутри APK с upstream provenance, ABI, набором патчей и fingerprint сертификата подписи.

Этого достаточно, чтобы будущий сайт принимал artifact после сборки, проверял SHA-256 и signer, а затем регистрировал его в приватном каталоге.

## Контракт будущего сайта

Сайт должен разделять авторизацию и хранение artifacts:

1. Пользователь проходит серверную авторизацию и получает короткоживущую session/entitlement.
2. API каталога возвращает metadata только для доступной пользователю сборки.
3. Download endpoint повторно проверяет entitlement и выдаёт короткоживущий одноразовый или подписанный URL.
4. Storage origin не должен иметь публичного listing или постоянного обходного URL.
5. Сервер перед выдачей сверяет SHA-256 и сертификат подписи с записью, созданной trusted build job.

Никакие access/refresh token, cookie, пароль keystore или Discord credential не должны записываться в APK, `danger-patch.json`, workflow artifact либо логи. Авторизация относится к скачиванию патченной сборки; работа `ympatcher --latest` с публичным upstream API остаётся отдельным контуром.

При подключении сайта меняется только consumer после trusted build: вместо ручного получения Actions artifact job отправляет APK, SHA-256 и извлечённую metadata в закрытый ingestion endpoint через GitHub OIDC или отдельный upload secret. Патчинг, provenance и signing pipeline при этом не требуют переписывания.
