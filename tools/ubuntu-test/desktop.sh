#!/bin/sh
# Starts a minimal desktop session on :99 (panel with tray, window manager, unlocked keyring).
export DISPLAY=:99
Xvfb :99 -screen 0 1440x900x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
sleep 1
eval "$(dbus-launch --sh-syntax)"
{ echo "export DISPLAY=:99"; echo "export DBUS_SESSION_BUS_ADDRESS=$DBUS_SESSION_BUS_ADDRESS"; } > /tmp/session.env
printf 'demo' | gnome-keyring-daemon --unlock --components=secrets >/tmp/keyring.log 2>&1
gnome-keyring-daemon --start --components=secrets >>/tmp/keyring.log 2>&1
xfwm4 >/tmp/xfwm4.log 2>&1 &
xfdesktop >/tmp/xfdesktop.log 2>&1 &
xfce4-panel >/tmp/panel.log 2>&1 &
echo DESKTOP_READY
sleep infinity
