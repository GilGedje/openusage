LiteLLM Usage tray app for Ubuntu — offline installer

What's inside
  packages/             The tray app and every library it needs (a small local apt repository)
  install.sh            Installer (run as the user, not with sudo)
  install.conf          Settings: your LiteLLM and status page addresses
  UBUNTU, ARCH          The Ubuntu version and CPU this folder is for
  SHA256SUMS            Integrity check

For the admin, once
  Edit install.conf: set LITELLM_URL and STATUS_URL (or leave STATUS_URL empty), then hand the
  folder to users.

For each user
  1. Run:  ./install.sh   and enter your password when asked.
  2. The panel opens at the top right: click Sign In, finish in your browser with your company
     SSO, and type the code the panel shows.
  The ring icon in the top bar shows how much of your budget you've used; click it (then Open)
  for the details.

Requirements
  The Ubuntu desktop version named in the UBUNTU file (there's one folder per version) and the CPU
  in ARCH. Nothing else: all libraries are in packages/. The installer installs only the ones the
  machine is missing, never removes anything, and doesn't touch the machine's apt sources.
  Nothing is downloaded from the internet. The app only talks to the addresses in install.conf.

Also available: the ccline folder adds usage to Claude Code's status line. Both share one sign-in.
