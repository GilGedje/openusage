# Ubuntu test harness (Docker)

Builds the Ubuntu installers the same way the Bundles workflow does, then installs them on a real
Ubuntu desktop running in Docker so you can click the tray and take screenshots. Works on any machine
with Docker (on Apple silicon it builds and tests arm64; GitHub builds x86_64).

## 1. Build the two installer folders (Ubuntu 22.04)

```sh
docker volume create ubuntu-build-cache
docker run --name ubuntu-build --rm -v "$PWD":/src \
  -v ubuntu-build-cache:/cache -v ubuntu-build-cargo:/root/.cargo -v ubuntu-build-rustup:/root/.rustup \
  ubuntu:22.04 sh /src/tools/ubuntu-test/build.sh
```

Output: `.ubuntu-test/dist/ccline-ubuntu-<arch>/`, `.ubuntu-test/dist/litellm-usage-tray-ubuntu-<arch>/`
and a `.tar.gz` of each. The first run takes ~10 minutes (Rust + Tauri); later runs are cached.

## 2. Start the test desktop (Ubuntu 24.04, Xfce panel with a tray, unlocked keyring)

```sh
docker build -t litellm-usage-desktop -f tools/ubuntu-test/Dockerfile.desktop tools/ubuntu-test
docker run -d --name ubuntu-desktop --add-host host.docker.internal:host-gateway \
  -v "$PWD/.ubuntu-test":/test:ro -v "$PWD/tools/ubuntu-test":/tools:ro \
  litellm-usage-desktop sh /tools/desktop.sh
```

GNOME (Ubuntu's default desktop) needs a GPU and doesn't run in Docker; Xfce's panel shows the same
tray icon and the panel window is identical.

## 3. Install, as the normal user `demo`

The container reaches a LiteLLM running on the host at `http://host.docker.internal:4000`.

```sh
docker exec ubuntu-desktop sh -c '. /tmp/session.env; sudo apt-get update -qq
  mkdir -p ~/dl && cd ~/dl && tar -xzf /test/dist/litellm-usage-tray-ubuntu-aarch64.tar.gz
  cd litellm-usage-tray-ubuntu-aarch64
  sed -i "s|https://litellm.example.internal|http://host.docker.internal:4000|; s|https://status.example.internal|https://status.mycompany.test|" install.conf
  ./install.sh'
```

Same for the `ccline-ubuntu-*` folder.

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
