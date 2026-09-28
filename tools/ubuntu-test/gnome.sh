#!/bin/sh
# Starts GNOME Shell (X11) on :99 with Ubuntu's AppIndicator extension and any user extensions
# already in ~/.local/share/gnome-shell/extensions enabled. Writes /tmp/session.env for docker exec.
# Screenshot (the compositor hides the screen from import/xwd):
#   gdbus call --session --dest org.gnome.Shell.Screenshot --object-path /org/gnome/Shell/Screenshot \
#     --method org.gnome.Shell.Screenshot.Screenshot false false /tmp/s.png
export DISPLAY=:99
# GNOME Shell wants a system bus, and uses logind when /run/systemd/seats exists (a container has
# no logind, so hide that and let the shell fall back to running without one).
sudo mkdir -p /run/dbus && sudo dbus-daemon --system --fork
[ -d /run/systemd/seats ] && sudo mv /run/systemd/seats /run/systemd/seats.off
Xvfb :99 -screen 0 1440x900x24 +extension GLX -nolisten tcp >/tmp/xvfb.log 2>&1 &
sleep 1
eval "$(dbus-launch --sh-syntax)"
{ echo "export DISPLAY=:99"; echo "export DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS"; } > /tmp/session.env
printf 'demo' | gnome-keyring-daemon --unlock --components=secrets >/tmp/keyring.log 2>&1
gnome-keyring-daemon --start --components=secrets >>/tmp/keyring.log 2>&1
USER_EXTS=$(ls "$HOME/.local/share/gnome-shell/extensions" 2>/dev/null | sed "s/.*/'&'/" | paste -sd, -)
gsettings set org.gnome.shell enabled-extensions "['ubuntu-appindicators@ubuntu.com'${USER_EXTS:+, $USER_EXTS}]"
gsettings set org.gnome.shell disable-user-extensions false
XDG_SESSION_TYPE=x11 gnome-shell --x11 --replace >/tmp/shell.log 2>&1 &
sleep 8
echo DESKTOP_READY
sleep infinity
