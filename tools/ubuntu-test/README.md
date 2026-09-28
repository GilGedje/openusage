# Ubuntu test harness (Docker)

Builds the Ubuntu installers the same way the Bundles workflow does, then installs them on a real
Ubuntu desktop running in Docker so you can click the tray and take screenshots. Works on any machine
with Docker (on Apple silicon it builds and tests arm64; GitHub builds x86_64).

## 1. Build every installer folder

```sh
sh tools/ubuntu-test/build-all.sh
```

Builds `ccline` (in ubuntu:20.04), the Tauri 2 tray `.deb` (22.04), the Tauri 1 tray `.deb` (20.04),
collects each release's offline packages inside a container of that release, and assembles
`.ubuntu-test/dist/ccline-ubuntu-<arch>/` and `.ubuntu-test/dist/quota-tray-ubuntu-<release>-<arch>/`
(+ `.tar.gz`). First run ~20–30 minutes; later runs are cached.

## 2. Start the test desktop (Ubuntu 24.04, Xfce panel with a tray, unlocked keyring)

```sh
docker build --build-arg UBUNTU=24.04 -t litellm-usage-desktop:24.04 -f tools/ubuntu-test/Dockerfile.desktop tools/ubuntu-test
docker run -d --name ubuntu-desktop --add-host host.docker.internal:host-gateway \
  -v "$PWD/.ubuntu-test":/test:ro -v "$PWD/tools/ubuntu-test":/tools:ro \
  litellm-usage-desktop:24.04 sh /tools/desktop.sh
# Offline test: cut the network before installing (reconnect later for sign-in)
docker network disconnect bridge ubuntu-desktop
```

GNOME (Ubuntu's default desktop) needs a GPU and doesn't run in Docker; Xfce's panel shows the same
tray icon and the panel window is identical.

## 3. Install, as the normal user `demo`

The container reaches a LiteLLM running on the host at `http://host.docker.internal:4000`.

```sh
docker exec ubuntu-desktop sh -c '. /tmp/session.env
  mkdir -p ~/dl && cd ~/dl && tar -xzf /test/dist/quota-tray-ubuntu-24.04-aarch64.tar.gz
  cd quota-tray-ubuntu-24.04-aarch64
  sed -i "s|https://litellm.example.internal|http://host.docker.internal:4000|; s|https://status.example.internal|https://status.mycompany.test|" install.conf
  ./install.sh'
```

Same for the `ccline-ubuntu-*` folder. After installing, check nothing is missing:
`ldd /usr/bin/litellm-usage | grep "not found"` and the same for
`/usr/lib/*/webkit2gtk-4.*/WebKit*Process` — both must print nothing. The image has no apt lists, so
apt can only use the folder. On 20.04 the desktop already has WebKitGTK 4.0; remove it first with
`sudo apt-get remove libwebkit2gtk-4.0-37 libjavascriptcoregtk-4.0-18 libayatana-appindicator3-1` for a
strict test.

## 4. Sign in, click, screenshot

- No browser in the container: install a stand-in that records the sign-in link, then open that link
  on the host (replace `host.docker.internal` with `localhost`) and type the code the panel shows:
  `docker exec -u root ubuntu-desktop sh -c 'printf "#!/bin/sh\necho \"\$@\" >> /tmp/opened.txt\n" > /usr/local/bin/xdg-open; chmod 755 /usr/local/bin/xdg-open'`
- Screenshot: `docker exec ubuntu-desktop sh -c '. /tmp/session.env; import -window root /tmp/s.png' && docker cp ubuntu-desktop:/tmp/s.png .`
- Click: `docker exec ubuntu-desktop sh -c '. /tmp/session.env; xdotool mousemove X Y click 1'`
  (the tray icon sits left of the clock in the top bar; on Linux it opens a menu → **Open**).
- Dark mode: `xfsettingsd &` then `xfconf-query -c xsettings -p /Net/ThemeName -s Adwaita-dark`, and
  restart the app.

Clean up: `docker rm -f ubuntu-desktop`.

## 5. Tray clicks on real GNOME (Ubuntu's own desktop)

The Xfce desktop above doesn't show how Ubuntu's GNOME handles tray clicks (its AppIndicator
extension turns a single left click into the menu). `Dockerfile.gnome` runs a real GNOME Shell (X11
session on Xvfb) with that extension:

```sh
cd tools/ubuntu-test && docker build --build-arg UBUNTU=20.04 -t quota-gnome:20.04 -f Dockerfile.gnome .
cd ../.. && docker run -d --name qg -v "$PWD":/src quota-gnome:20.04 sh -c '
  sudo apt-get update -qq && sudo apt-get install -y -qq /src/.ubuntu-test/build/tray-legacy.deb
  E=$HOME/.local/share/gnome-shell/extensions/quota-tray-click@exodus.ai; mkdir -p $E
  cp /src/installer/linux/tray/gnome-extension/legacy/* $E/     # modern/ for GNOME 45+ (24.04, 26.04)
  /src/tools/ubuntu-test/gnome.sh'
docker exec qg sh -c '. /tmp/session.env; litellm-usage >/tmp/app.log 2>&1 &'
docker exec qg sh /src/tools/ubuntu-test/gnome-clicks.sh 1312      # the icon's x (20.04: 1312)
```

Screenshots: `import` shows black under GNOME's compositor; use the Shell's screenshot service (see
gnome.sh; GNOME 41+ refuses it, so check window state with `xwininfo` instead). 26.04 (GNOME 50) has
no X11 session, and Wayland sessions aren't covered here yet.
