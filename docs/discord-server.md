# Discord server setup

The idempotent setup script creates and updates the YM Patcher server layout without deleting unrelated channels. It also maintains welcome, rules, and release embeds, creates separate Discord webhooks for GitHub activity and releases, and configures the corresponding repository webhooks.

Required bot permissions in guild `1544798877260193814`:

- Manage Channels
- Manage Webhooks
- View Channels
- Send Messages
- Embed Links
- Read Message History

Use the bot belonging to the YM Patcher Discord application. Never paste its token into chat, commit it, or pass it as a command-line argument. Put both tokens into process environment variables using a secure local prompt, then run:

```powershell
$env:DISCORD_BOT_TOKEN = Read-Host 'Discord bot token'
$env:DISCORD_APPLICATION_ID = '1522963615437553694'
$env:GITHUB_TOKEN = gh auth token
$env:DISCORD_GUILD_ID = '1544798877260193814'
$env:GITHUB_REPOSITORY = 'pyanexya/ympatcher'
node scripts/discord_server.mjs bootstrap
Remove-Item Env:DISCORD_BOT_TOKEN, Env:GITHUB_TOKEN
```

Run `node scripts/discord_server.mjs test` with `DISCORD_BOT_TOKEN` set to send one test message through the managed GitHub webhook. Re-running `bootstrap` updates managed channels and messages instead of duplicating them.
