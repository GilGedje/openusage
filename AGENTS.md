# AGENTS.md

A cross-platform (Windows and Ubuntu, macOS if possible) usage view for people who use Claude Code through a LiteLLM
proxy. It shows the user's LiteLLM budget, spend, and per-model usage. Claude Code is only the harness;
all usage data comes from the user's LiteLLM proxy, not from Anthropic's API.

This file documents the engineering conventions for the project. Read it before contributing.

## Agent Instructions

AGENTS.md is the source of truth for agent instructions in this repository. CLAUDE.md files may only point to the nearest AGENTS.md file with `@AGENTS.md`; do not add guidance, duplicate instructions, or project rules to CLAUDE.md.

> **Repository note:** This repository started as a fork of OpenUsage (MIT). All of the original app code has been removed. It is not the official OpenUsage and must not use the OpenUsage name or logo.

## Releases

- **Never increase the version number on your own initiative — always ask for explicit approval first.** Propose the number and wait for explicit sign-off before tagging or cutting a release.
- Never leave a release in Draft, and never ship blank notes.

## Pull Requests

Every PR description must follow this structure so reviewers can skim it quickly:

- **TL;DR** — open with a one- or two-sentence plain-English summary of the change.
- **What was happening** — plain-English bullet points describing the prior behavior, bug, or gap that motivated the change.
- **What this changes** — bullet points describing what the PR actually changes.
- **Heads-up** (optional) — noteworthy things a reviewer or future maintainer should consider (risks, follow-ups, trade-offs).
- **Tests** (optional) — how the change was verified.
- **Screenshots** (optional in general, but **required for any PR that makes a visual change**) — images of the affected UI after the change.

## Documentation

- Logic changes must update any docs that describe the affected behavior.
- Keep docs simple, less-technical, and easy to skim; exclude visual design details.

## Code Conventions

- Add a regression test when fixing a bug, where it fits.
- Keep files under ~500 LOC; split or refactor as needed.
- No new dependencies without justification.
- Every change must work on Windows and Ubuntu (macOS too where possible). Never hardcode OS-specific paths or tools without a per-OS branch.

## Error Handling

Always fail loudly into error logging and show friendly errors to the user. Do not add silent fallbacks that hide real problems. Only validate at system boundaries (user input, external APIs such as the LiteLLM proxy); trust internal code and framework guarantees.

## UI

- Styling comes from `design/theme.css` (colors, light/dark, density, card and meter styles). Use its tokens instead of hardcoding colors or sizes.
- Use title case for any hardcoded copy used as a title.
- Match the existing design language: opaque tray, borderless grouped cards, thin capsule meters in blue / yellow / red.
- Only add tooltips when explicitly asked to. Don't add them proactively to new controls.
