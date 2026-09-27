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
`.ubuntu-test/dist/ccline-ubuntu-<arch>/` and `.ubuntu-test/dist/litellm-usage-tray-ubuntu-<release>-<arch>/`
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
  mkdir -p ~/dl && cd ~/dl && tar -xzf /test/dist/litellm-usage-tray-ubuntu-24.04-aarch64.tar.gz
  cd litellm-usage-tray-ubuntu-24.04-aarch64
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
