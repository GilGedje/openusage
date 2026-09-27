LiteLLM Usage for Windows — early build (programs only, no installer yet)

What's inside
  ccline\ccline.exe              Claude Code status line + command-line tool
  tray\litellm-usage.exe         Tray app

These are built and tested automatically on Windows, but haven't been tried by hand on a real
Windows machine yet. A proper offline installer (.msi with WebView2 included) is next.

Requirements
  Windows 10 (1809+) or 11, 64-bit.
  The tray app needs Microsoft WebView2 Runtime. Windows 11 always has it; Windows 10 usually has
  it through Microsoft Edge. Check: Settings > Apps > installed apps > "Microsoft Edge WebView2
  Runtime". If it's missing on an air-gapped machine, wait for the .msi installer (it includes it).
  ccline needs nothing else.
  Nothing is downloaded from the internet. Both only talk to your LiteLLM (and status page).

Install ccline (PowerShell, as the user — replace the address)
  $dir = "$env:LOCALAPPDATA\Programs\ccline"
  New-Item -ItemType Directory -Force $dir | Out-Null
  Copy-Item .\ccline\ccline.exe $dir
  & "$dir\ccline.exe" setup --url https://your-litellm-proxy
  & "$dir\ccline.exe" login
  -> "setup" saves the address and adds the status line to %USERPROFILE%\.claude\settings.json
     (backup kept as settings.json.bak-ccline). "login" opens your browser for SSO; type the code
     shown in PowerShell.

Install the tray app (PowerShell, as the user — replace the addresses)
  $dir = "$env:LOCALAPPDATA\Programs\LiteLLM Usage"
  New-Item -ItemType Directory -Force $dir | Out-Null
  Copy-Item .\tray\litellm-usage.exe $dir
  & "$dir\litellm-usage.exe" --configure --url https://your-litellm-proxy --status-url https://your-status-page
  Start-Process "$dir\litellm-usage.exe"
  -> "--configure" saves the addresses (it prints nothing on Windows; exit code 0 means OK).
     Click the ring icon in the taskbar's notification area, then Sign In.
  Start at login (optional):
  $s = (New-Object -ComObject WScript.Shell).CreateShortcut("$([Environment]::GetFolderPath('Startup'))\LiteLLM Usage.lnk")
  $s.TargetPath = "$dir\litellm-usage.exe"; $s.Save()

Both share one sign-in (stored in Windows Credential Manager) and one settings file
(%APPDATA%\litellm-usage\config.json).

Useful commands
  ccline status        show budget and usage now
  ccline login         sign in again
  ccline logout        sign out
