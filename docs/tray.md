# Tray app

A tray icon that shows how much of your LiteLLM budget you've used, with a popup panel for the
details. It shares its sign-in and cached numbers with `ccline`, so the two never fetch twice.

## The tray icon

- A ring that fills as you spend your budget. On macOS it follows the menu bar's light/dark style
  and shows the percentage next to it; on Windows and Ubuntu the ring turns yellow from 75% used and
  red from 90%.
- Hovering shows `LiteLLM: $31.26 of $50.00 (63%)`.
- Click it to open the panel (on Ubuntu, pick **Open** from its menu). Its menu also has **Refresh**
  and **Quit**.

## The panel

- **Cost** — what you spent **Today**, **Yesterday**, or over the last **30 Days**, as a donut split
  by model with the amount per model. The five biggest models get their own color; the rest are
  grouped as **Other**. The 30 Days view adds a bar per day. The panel remembers your choice.
- **LiteLLM** (with your user name):
  - **Budget** — how much of your budget is left, when it resets, and a tick showing where you'd be
    if you spent evenly across the budget window. If you're spending fast enough to run out before the
    reset, it warns **Limit in …** instead.
  - **Today / Yesterday / Last 30 Days** — spend and tokens.
- **Dashboard** — opens LiteLLM's own Usage page. **Status** — opens your organization's status page
  (shown only when one is set, see below).
- **Footer** — when the numbers were last updated, **Refresh Now**, **Sign Out**, **Quit**.

If LiteLLM can't be reached, the last numbers stay up with a note saying how old they are.

## Signing in

Same SSO sign-in as `ccline login`: enter your LiteLLM address (pre-filled from Claude Code's
`ANTHROPIC_BASE_URL` or your last sign-in), click **Sign In**, finish in the browser, and type the
code the panel shows. If you're in several teams, the panel asks which one Claude Code uses.

## Status page

Set it once per machine, either as `"status_url": "https://status.example.com"` in the settings file
(`litellm-usage/config.json` in the OS config folder — on macOS `~/Library/Application Support/`) or
with the `LITELLM_USAGE_STATUS_URL` environment variable (which wins). Without it, the Status link is
hidden.

## Refreshing

The app checks every 15 seconds and fetches when the shared numbers are over a minute old — whether
`ccline` or the tray fetched them last. The refresh button in the panel fetches right away.

## Running from source

`cargo run -p litellm-usage-tray`. Set `LITELLM_USAGE_OPEN_PANEL=1` to open the panel at launch.
On macOS, each new build asks once for Keychain access — choose **Always Allow**.
