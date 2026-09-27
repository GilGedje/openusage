ccline for Ubuntu — Claude Code status line, offline installer

What's inside
  ccline                Status line + command-line tool
  install.sh            Installer (run as the user; no sudo needed)
  install.conf          Settings: your LiteLLM address
  SHA256SUMS, ARCH      Integrity check and CPU type

For the admin, once
  Edit install.conf: set LITELLM_URL, then hand the folder to users.

For each user
  1. Run:  ./install.sh
  2. Your browser opens: sign in with your company SSO and type the code shown in the terminal.
  Claude Code now shows the model, context, and your LiteLLM budget in its status line.

Requirements
  Ubuntu 22.04 or newer. Nothing else. Nothing is downloaded from the internet; ccline only talks
  to the address in install.conf.

Useful commands
  ccline status        show budget and usage now
  ccline login         sign in again
  ccline logout        sign out

Also available: the litellm-usage-tray folder adds a tray icon with a usage panel. Both share
one sign-in.
