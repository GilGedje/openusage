# LiteLLM API

How the app signs in to a user's LiteLLM proxy and reads their budget and usage. Everything here was
checked against a live proxy (LiteLLM 1.99.1 Enterprise, SSO on) with an SSO login — no API key.

## Signing in (SSO)

LiteLLM has a device-style login for desktop and command-line apps. No API key is created or stored.

1. `POST /sso/cli/start` (no auth, empty JSON body) returns:
   - `login_id` — e.g. `cli-…`
   - `poll_secret` — keep private; only the app needs it
   - `user_code` — e.g. `ABCD-1234`, shown to the user
   - `expires_in` — 600 seconds to finish signing in
2. Open the browser at `/sso/key/generate?source=litellm-cli&key=<login_id>`. The user signs in with SSO
   and types the code.
3. Poll `GET /sso/cli/poll/<login_id>` with header `x-litellm-cli-poll-secret: <poll_secret>` every few
   seconds:
   - `{"status": "pending"}` — keep waiting
   - `{"status": "ready", "key": "<token>", "user_id": …, "team_id": …, "teams": […]}` — done
   - When the user is in more than one team, the first ready answer has `requires_team_selection: true`
     and a `team_details` list instead of a key; poll again with `?team_id=<id>` to get the token.
   - The token is single-use to collect: once returned, the login session is deleted.
4. Send the token on every request as `Authorization: Bearer <token>`.

The token lasts 24 hours by default. The proxy admin can raise it with
`LITELLM_CLI_JWT_EXPIRATION_HOURS`. There is no refresh token: when a request returns `401`, sign in
again. With `general_settings.allow_cli_sso_verification_uri_complete: true`, `/sso/cli/start` also
returns `verification_uri_complete` (a link with the code filled in), so signing in again is one click.

The token can also run models, so it's stored in the OS secure storage (macOS Keychain, Windows
Credential Manager, Secret Service / keyring on Ubuntu), never in a plain file.

## What works with the SSO token

| Endpoint | Result | Use |
|---|---|---|
| `GET /v2/user/info` | ✅ | Budget: `spend`, `max_budget`, `budget_duration`, `budget_reset_at`, per-model budgets (`model_max_budget`, `model_max_budget_usage`), `user_role`, `teams` |
| `GET /user/daily/activity/aggregated?start_date=&end_date=` | ✅ | Daily usage with breakdowns (see below) |
| `GET /user/daily/activity` | ✅ | Same, paginated |
| `GET /user/spend/report?start_date=&end_date=` | ✅ | Spend per key with per-model totals |
| `GET /user/info` | ✅ | Older, heavier version of `/v2/user/info` that also lists the user's keys |
| `GET /v1/models` | ✅ | Model names the user can call |
| `GET /model/info` | ✅ | Each model name → real backend model and its prices |
| `GET /spend/logs/v2` | ✅ | Individual requests, paginated |
| `GET /key/info` | ❌ 404 | The SSO token isn't a stored key — use `/v2/user/info` instead |
| `GET /key/spend/report` | ⚠️ empty | Same reason |

Dates are `YYYY-MM-DD`. Add `include_current_utc_day=true` to include today.

## Reading the budget

From `/v2/user/info`:

- `spend` counts only the current budget window (it resets to 0 at `budget_reset_at`).
- `max_budget` is `null` when there's no limit.
- `budget_reset_at` is when the window ends next, not when it started.

Users in a team may also have a team budget: `GET /team/{team_id}/members/me` (not yet tested — the test
user had no team).

## Reading daily usage

Each day in `results` has `metrics` (the day's totals) and `breakdown`. Each breakdown entry has the
same `metrics` shape: `spend`, `prompt_tokens`, `completion_tokens`, `cache_read_input_tokens`,
`cache_creation_input_tokens`, `total_tokens`, `api_requests`, `successful_requests`, `failed_requests`.

- `breakdown.model_groups` — the model names users pick in Claude Code (`sonnet`, `claude-sonnet-4-6`,
  `haiku`). **Show these to users.**
- `breakdown.models` — the real backend models behind them (e.g. `gemini/gemini-2.5-pro`).
- `breakdown.providers`, `breakdown.api_keys` (hashed) — also available.

`metadata` holds totals for the whole date range (`total_spend`, `total_tokens`, request counts).
