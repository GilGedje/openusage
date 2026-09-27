LiteLLM Usage tray app for Ubuntu — offline installer

What's inside
  litellm-usage_*.deb   The tray app
  install.sh            Installer (run as the user, not with sudo)
  install.conf          Settings: your LiteLLM and status page addresses
  SHA256SUMS, ARCH      Integrity check and CPU type

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
  Ubuntu 22.04 or 24.04 desktop. The app's libraries (WebKitGTK 4.1, GTK 3, AppIndicator) come
  from your apt mirror; a standard Ubuntu desktop already has most of them.
  Nothing is downloaded from the internet. The app only talks to the addresses in install.conf.

Also available: the ccline folder adds usage to Claude Code's status line. Both share one sign-in.
