#!/usr/bin/env node

const DISCORD_API = 'https://discord.com/api/v10'
const GITHUB_API = 'https://api.github.com'
const BOT_TOKEN = process.env.DISCORD_BOT_TOKEN?.trim()
const GUILD_ID = process.env.DISCORD_GUILD_ID?.trim() || '1544798877260193814'
const APPLICATION_ID = process.env.DISCORD_APPLICATION_ID?.trim() || '1522963615437553694'
const GITHUB_REPOSITORY = process.env.GITHUB_REPOSITORY?.trim() || 'pyanexya/ympatcher'
const GITHUB_TOKEN = process.env.GITHUB_TOKEN?.trim()
const command = process.argv[2] || 'bootstrap'

if (!BOT_TOKEN) fail('DISCORD_BOT_TOKEN is required')
if (!/^\d{17,20}$/.test(GUILD_ID)) fail('DISCORD_GUILD_ID must be a Discord snowflake')
if (!/^[\w.-]+\/[\w.-]+$/.test(GITHUB_REPOSITORY)) fail('GITHUB_REPOSITORY must be owner/repo')

const READ_ONLY_DENY = '2048'
const VIEW_CHANNEL_ALLOW = '1024'
const managedFooter = 'YM Patcher • managed setup'

const layout = [
  {
    name: '━━ ИНФОРМАЦИЯ ━━',
    channels: [
      { name: 'добро-пожаловать', topic: 'Начните здесь: сайт, GitHub и Discord YM Patcher.', readOnly: true },
      { name: 'правила', topic: 'Короткие правила сообщества YM Patcher.', readOnly: true },
      { name: 'анонсы', topic: 'Важные новости проекта и объявления команды.', readOnly: true },
      { name: 'релизы', topic: 'Новые Stable и Beta сборки, версии и changelog.', readOnly: true, webhook: 'YM Patcher • Releases' },
    ],
  },
  {
    name: '━━ СООБЩЕСТВО ━━',
    channels: [
      { name: 'общение', topic: 'Общее общение участников YM Patcher.' },
      { name: 'помощь', topic: 'Помощь с установкой, обновлением и настройкой патча.', slowmode: 5 },
      { name: 'баг-репорты', topic: 'Ошибки: версия, ABI, устройство, Android и шаги воспроизведения.', slowmode: 15 },
      { name: 'предложения', topic: 'Идеи и предложения по развитию проекта.', slowmode: 15 },
    ],
  },
  {
    name: '━━ РАЗРАБОТКА ━━',
    channels: [
      { name: 'beta-тестирование', topic: 'Обсуждение Beta-сборок и раннее тестирование.' },
      { name: 'разработка', topic: 'Техническое обсуждение ympatcher и вклад в проект.' },
      { name: 'github', topic: 'Автоматические события из pyanexya/ympatcher.', readOnly: true, webhook: 'YM Patcher • GitHub' },
    ],
  },
  {
    name: '━━ ГОЛОСОВЫЕ ━━',
    channels: [
      { name: 'Музыка', type: 2 },
      { name: 'Разговорная', type: 2 },
    ],
  },
]

const me = await discord('/users/@me')
if (me.id !== APPLICATION_ID) {
  fail(`The token belongs to bot ${me.id}, expected the YM Patcher application bot ${APPLICATION_ID}`)
}
const guild = await discord(`/guilds/${GUILD_ID}`)
const channels = await discord(`/guilds/${GUILD_ID}/channels`)
const managed = new Map()

for (const section of layout) {
  const category = await ensureCategory(section.name)
  for (const definition of section.channels) {
    const channel = await ensureChannel(category, definition)
    managed.set(definition.name, channel)
  }
}

await ensureInformationPosts()

const githubChannel = requiredChannel('github')
const releasesChannel = requiredChannel('релизы')
const githubWebhook = await ensureWebhook(githubChannel.id, 'YM Patcher • GitHub')
const releasesWebhook = await ensureWebhook(releasesChannel.id, 'YM Patcher • Releases')

if (command === 'bootstrap') {
  if (GITHUB_TOKEN) {
    await configureGitHubWebhook('YM Patcher Discord feed', githubWebhook, ['push', 'pull_request', 'issues'])
    await configureGitHubWebhook('YM Patcher Discord releases', releasesWebhook, ['release'])
  } else {
    console.log('Discord configured. Set GITHUB_TOKEN to configure repository webhooks.')
  }
  console.log(`Discord server configured: ${guild.name} (${GUILD_ID})`)
} else if (command === 'test') {
  await executeWebhook(githubWebhook, {
    username: 'YM Patcher • GitHub',
    embeds: [{
      title: 'Интеграция GitHub готова',
      description: 'События репозитория будут появляться в этом канале автоматически.',
      color: 0xF5C518,
      url: `https://github.com/${GITHUB_REPOSITORY}`,
      footer: { text: managedFooter },
      timestamp: new Date().toISOString(),
    }],
  })
  console.log('Test notification sent.')
} else {
  fail(`Unknown command: ${command}. Use bootstrap or test.`)
}

async function ensureCategory(name) {
  const existing = channels.find((channel) => channel.type === 4 && channel.name === name)
  if (existing) return existing
  const created = await discord(`/guilds/${GUILD_ID}/channels`, {
    method: 'POST',
    body: { name, type: 4 },
  })
  channels.push(created)
  return created
}

async function ensureChannel(category, definition) {
  const type = definition.type ?? 0
  let channel = channels.find((item) => item.type === type && item.name === definition.name && item.parent_id === category.id)
  if (!channel) {
    channel = await discord(`/guilds/${GUILD_ID}/channels`, {
      method: 'POST',
      body: {
        name: definition.name,
        type,
        parent_id: category.id,
        topic: type === 0 ? definition.topic : undefined,
        rate_limit_per_user: type === 0 ? definition.slowmode || 0 : undefined,
      },
    })
    channels.push(channel)
  } else if (type === 0 && (channel.topic !== definition.topic || channel.rate_limit_per_user !== (definition.slowmode || 0))) {
    channel = await discord(`/channels/${channel.id}`, {
      method: 'PATCH',
      body: { topic: definition.topic, rate_limit_per_user: definition.slowmode || 0 },
    })
  }

  if (type === 0 && definition.readOnly) {
    await discord(`/channels/${channel.id}/permissions/${GUILD_ID}`, {
      method: 'PUT',
      body: { type: 0, allow: VIEW_CHANNEL_ALLOW, deny: READ_ONLY_DENY },
      empty: true,
    })
  }
  return channel
}

async function ensureInformationPosts() {
  await ensureManagedMessage(requiredChannel('добро-пожаловать').id, 'welcome', {
    title: 'Добро пожаловать в YM Patcher',
    description: 'Android-патчер для Яндекс Музыки с Developer experiments и Discord Rich Presence. Presence входит в APK, но включается пользователем и по умолчанию выключен.',
    color: 0xF5C518,
    fields: [
      { name: 'Скачать', value: '[music.pyanexy.cc](https://music.pyanexy.cc)', inline: true },
      { name: 'Исходный код', value: `[GitHub](${'https://github.com/' + GITHUB_REPOSITORY})`, inline: true },
      { name: 'С чего начать', value: 'Прочитайте правила, выберите Stable или Beta на сайте и укажите ABI вашего устройства.' },
    ],
  })

  await ensureManagedMessage(requiredChannel('правила').id, 'rules', {
    title: 'Правила сообщества',
    description: [
      '1. Общайтесь уважительно и без спама.',
      '2. Не публикуйте токены, пароли, cookie, ключи подписи и другие секреты.',
      '3. Для баг-репорта укажите версию сборки, ABI, Android и шаги воспроизведения.',
      '4. Не распространяйте APK проекта в обход авторизованной выдачи.',
      '5. Используйте тематические каналы — так ответы находятся быстрее.',
    ].join('\n'),
    color: 0xF5C518,
  })

  await ensureManagedMessage(requiredChannel('релизы').id, 'releases', {
    title: 'Релизы YM Patcher',
    description: 'Здесь публикуются Stable и Beta обновления. APK скачивается только через сайт после Discord-авторизации.',
    color: 0x4ADE80,
    fields: [
      { name: 'Stable', value: 'Проверенная сборка для повседневного использования.', inline: true },
      { name: 'Beta', value: 'Новые изменения раньше Stable; возможны отдельные ошибки.', inline: true },
      { name: 'ABI', value: '`arm64-v8a` — современные устройства\n`armeabi-v7a` — старые 32-битные устройства' },
    ],
  })
}

async function ensureManagedMessage(channelId, marker, embed) {
  const messages = await discord(`/channels/${channelId}/messages?limit=100`)
  const footerText = `${managedFooter} • ${marker}`
  const existing = messages.find((message) => message.author?.id === me.id && message.embeds?.some((item) => item.footer?.text === footerText))
  const body = { embeds: [{ ...embed, footer: { text: footerText } }], allowed_mentions: { parse: [] } }
  if (existing) {
    await discord(`/channels/${channelId}/messages/${existing.id}`, { method: 'PATCH', body })
  } else {
    await discord(`/channels/${channelId}/messages`, { method: 'POST', body })
  }
}

async function ensureWebhook(channelId, name) {
  const webhooks = await discord(`/channels/${channelId}/webhooks`)
  const existing = webhooks.find((webhook) => webhook.type === 1 && webhook.name === name && webhook.token)
  if (existing) return existing
  return discord(`/channels/${channelId}/webhooks`, { method: 'POST', body: { name } })
}

async function configureGitHubWebhook(label, discordWebhook, events) {
  if (!discordWebhook.token) fail(`Discord did not return a token for webhook: ${label}`)
  const [owner, repo] = GITHUB_REPOSITORY.split('/')
  const url = `${DISCORD_API}/webhooks/${discordWebhook.id}/${discordWebhook.token}/github`
  const hooks = await github(`/repos/${owner}/${repo}/hooks?per_page=100`)
  const existing = hooks.find((hook) => hook.config?.url === url || hook.config?.url?.includes(`/webhooks/${discordWebhook.id}/`))
  const body = {
    name: 'web',
    active: true,
    events,
    config: { url, content_type: 'json', insecure_ssl: '0' },
  }
  if (existing) {
    await github(`/repos/${owner}/${repo}/hooks/${existing.id}`, { method: 'PATCH', body })
  } else {
    await github(`/repos/${owner}/${repo}/hooks`, { method: 'POST', body })
  }
  console.log(`GitHub webhook configured: ${label}`)
}

async function executeWebhook(webhook, body) {
  if (!webhook.token) fail('Discord webhook token is unavailable')
  const response = await fetch(`${DISCORD_API}/webhooks/${webhook.id}/${webhook.token}?wait=true`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ...body, allowed_mentions: { parse: [] } }),
  })
  if (!response.ok) fail(`Discord webhook request failed (${response.status})`)
}

function requiredChannel(name) {
  const channel = managed.get(name)
  if (!channel) fail(`Managed channel not found: ${name}`)
  return channel
}

async function discord(path, options = {}) {
  return request(`${DISCORD_API}${path}`, {
    ...options,
    headers: { Authorization: `Bot ${BOT_TOKEN}`, ...options.headers },
  })
}

async function github(path, options = {}) {
  if (!GITHUB_TOKEN) fail('GITHUB_TOKEN is required to configure repository webhooks')
  return request(`${GITHUB_API}${path}`, {
    ...options,
    headers: {
      Authorization: `Bearer ${GITHUB_TOKEN}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': 'ympatcher-discord-setup',
      ...options.headers,
    },
  })
}

async function request(url, options = {}) {
  const headers = { ...options.headers }
  const body = options.body === undefined ? undefined : JSON.stringify(options.body)
  if (body !== undefined) headers['Content-Type'] = 'application/json'

  for (let attempt = 0; attempt < 4; attempt++) {
    const response = await fetch(url, { method: options.method || 'GET', headers, body })
    if (response.status === 429) {
      const rateLimit = await response.json().catch(() => ({}))
      const waitMs = Math.min(10_000, Math.max(250, Number(rateLimit.retry_after || 1) * 1000))
      await new Promise((resolve) => setTimeout(resolve, waitMs))
      continue
    }
    if (!response.ok) {
      const detail = await response.text()
      fail(`Request failed (${response.status} ${response.statusText}): ${redact(detail)}`)
    }
    if (options.empty || response.status === 204) return null
    return response.json()
  }
  fail('Request was rate-limited too many times')
}

function redact(value) {
  if (!value) return 'no response body'
  let safe = String(value)
  if (BOT_TOKEN) safe = safe.replaceAll(BOT_TOKEN, '[REDACTED]')
  if (GITHUB_TOKEN) safe = safe.replaceAll(GITHUB_TOKEN, '[REDACTED]')
  return safe.slice(0, 1000)
}

function fail(message) {
  console.error(message)
  process.exit(1)
}
