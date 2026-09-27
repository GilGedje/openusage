# ccline — Claude Code status line

Shows your LiteLLM budget and usage at the bottom of Claude Code:

```
▰▰▰▱▱ $31.26/$50.00 63% · resets 3d · today $0.00 · claude-3-5-sonnet $15.83/30d
```

- **Budget meter** — spend in the current budget window against your limit. Blue below 75%, yellow from
  75%, red from 90%. Without a limit it reads `$12.40 spent · no limit`.
- **resets** — time until the budget window resets.
- **today** — today's spend.
- **Current model** — what the model Claude Code is using has cost over the last 30 days (shown when
  LiteLLM has usage under that exact model name).

When the terminal is narrow, the least important parts drop off first (model, then reset, then today).

## Setup

1. Put the `ccline` binary on your PATH (e.g. `~/.local/bin/ccline`).
2. Sign in: `ccline login --url http://your-litellm-proxy:4000`. Without `--url`, it uses
   `ANTHROPIC_BASE_URL`, then the proxy you signed in to last time. Your browser opens; sign in with
   SSO and enter the code shown in the terminal.
3. Add to `~/.claude/settings.json`:

   ```json
   "statusLine": { "type": "command", "command": "~/.local/bin/ccline", "refreshInterval": 30 }
   ```

   On Windows, use forward slashes in the path (e.g. `C:/Users/you/bin/ccline.exe`).

## Commands

| Command | What it does |
|---|---|
| `ccline` | Prints the status line (Claude Code runs this) |
| `ccline login [--url URL]` | Signs in with SSO. If you're in several teams, asks which one |
| `ccline logout` | Forgets the sign-in and the cached usage |
| `ccline status` | Fetches now and prints budget, today, 30 days, and spend per model |

## How it stays fast

The status line never waits on the network. It prints the last saved numbers instantly, and when
they're more than a minute old it starts a background refresh for next time. Only one refresh runs at
a time, even with several Claude Code windows open.

If a refresh fails, the old numbers stay on screen with a warning and their age
(`⚠ can't reach proxy · 5m old`). Details go to the error log.

## Sign-in and storage

- Uses LiteLLM's SSO login — no API key is created. See [litellm-api.md](litellm-api.md).
- The sign-in token lives in the OS secure storage: Keychain (macOS), Credential Manager (Windows),
  Secret Service / GNOME Keyring (Ubuntu). It's never written to a file.
- The token expires after the proxy's configured lifetime (24 hours by default). The status line then
  shows `LiteLLM: signed out · run ccline login`.
- Files (no secrets): settings in the OS config folder, cached usage and `error.log` in the OS cache
  folder, both under `litellm-usage/` (on macOS: `~/Library/Application Support/litellm-usage/` and
  `~/Library/Caches/litellm-usage/`).

## Troubleshooting

- **`secure storage unavailable`** on Ubuntu — the Secret Service isn't running (common over SSH or on
  servers without a desktop). Sign in from a desktop session.
- **macOS asks for Keychain access** after installing a new build — choose **Always Allow**.
- **`localhost` proxy** — ccline connects to `127.0.0.1` for `localhost`, like browsers do, so it works
  even when the hosts file has no `localhost` entry.
