# Участие в разработке

Создавайте ветки от `main` и открывайте pull request в `main`. Релизные теги: `vX.Y.Z`, предварительные: `vX.Y.Z-dev.N`, `vX.Y.Z-beta.N`, `vX.Y.Z-rc.N`. Канал исходного клиента выбирается отдельно от Git-ветки.

Перед pull request:

```sh
python scripts/check_publication.py
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all --locked
cargo build --release --locked
```

Для изменений Android-кода соберите затронутый Gradle-модуль и укажите, была ли проверка на устройстве. Не выдавайте сборку за проверку запуска.

Не добавляйте клиентские APK, SDK, ключи, токены, cookies, webhooks, дампы аккаунтов или выгрузки содержимого чужого приложения. Используйте синтетические fixtures. Сообщения об ошибках должны содержать минимальный обезличенный logcat и шаги воспроизведения.
