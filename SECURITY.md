# Security policy

Не публикуйте в Issues APK, signing keystore, `signing.json`, токены, cookies или данные аккаунта.

Уязвимости можно отправить приватно через [GitHub Security Advisories](https://github.com/pyanexya/ympatcher/security/advisories/new). В сообщении укажите версию `ympatcher`, ОС, версию Java и минимальные шаги воспроизведения без персональных данных.

Исправления безопасности готовятся отдельно и включаются в `main`. Поддерживается текущая версия инструмента. Если private reporting на GitHub недоступен, не публикуйте exploit или секреты в обычном Issue; создайте запрос на включение private reporting без чувствительных подробностей.

В отчёте не публикуйте Discord webhook, Google/device credentials, private source URL или полный dataminer fragment, если он может содержать секрет. Workflows должны маскировать secrets и не включать их в command output.
