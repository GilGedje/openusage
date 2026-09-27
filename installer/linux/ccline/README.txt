ccline (Quota by Exodus.Ai) — Claude Code status line for Ubuntu, offline installer

What's inside
  ccline                Status line + command-line tool
  install.sh            Installer (run as the user; no sudo needed)
  install.conf          Settings: your LiteLLM address
  SHA256SUMS, ARCH      Integrity check and CPU type

For the admin, once
  If LiteLLM uses your organization's own certificate, set CA_CERT in install.conf (a path on the
  machines, or put the CA file in this folder and give its name). Then edit the rest of install.conf: set LITELLM_URL, then hand the folder to users.

For each user
  1. Run:  ./install.sh
  2. Your browser opens: sign in with your company SSO and type the code shown in the terminal.
  Claude Code now shows the model, context, and your LiteLLM budget in its status line.

Requirements
  Ubuntu 20.04 or newer (any CPU matching ARCH). Nothing else. Nothing is downloaded from the internet; ccline only talks
  to the address in install.conf.

Useful commands
  ccline status        show budget and usage now
  ccline login         sign in again
  ccline logout        sign out

Also available: the quota-tray folder adds a tray icon with a usage panel. Both share
one sign-in.
