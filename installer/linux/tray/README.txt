Quota by Exodus.Ai — tray app for Ubuntu, offline installer

What's inside
  packages/             The tray app and every library it needs (a small local apt repository)
  install.sh            Installer (run as the user, not with sudo)
  install.conf          Settings: your LiteLLM and status page addresses
  gnome-extension/      Small GNOME Shell add-on so a single click on the icon opens the panel
  UBUNTU, ARCH          The Ubuntu version and CPU this folder is for
  SHA256SUMS            Integrity check

For the admin, once
  If LiteLLM uses your organization's own certificate, set CA_CERT in install.conf (a path on the
  machines, or put the CA file in this folder and give its name). Then edit the rest of install.conf: set LITELLM_URL and STATUS_URL (or leave STATUS_URL empty), then hand the
  folder to users.

For each user
  1. Run:  ./install.sh   and enter your password when asked.
  2. The panel opens at the top right: click Sign In, finish in your browser with your company
     SSO, and type the code the panel shows.
  3. Log out and back in once. From then on a single click on the ring icon in the top bar opens
     the panel, and clicking anywhere else closes it. (Until then: click the icon, then Open.)
  The ring shows how much of your budget you've used; right click it for Refresh, Change LiteLLM
  URL… and Quit.

Requirements
  The Ubuntu desktop version named in the UBUNTU file (there's one folder per version) and the CPU
  in ARCH. Nothing else: all libraries are in packages/. The installer installs only the ones the
  machine is missing, never removes anything, and doesn't touch the machine's apt sources.
  Nothing is downloaded from the internet. The app only talks to the addresses in install.conf.

Also available: the ccline folder adds usage to Claude Code's status line. Both share one sign-in.
