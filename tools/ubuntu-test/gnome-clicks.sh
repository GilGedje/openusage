#!/bin/sh
# Tray click test on a GNOME desktop from gnome.sh (X11 session), with the tray app running and the
# tray-click extension enabled. Usage: docker exec <container> sh /src/tools/ubuntu-test/gnome-clicks.sh <icon x>
# (find the icon x by clicking along y=12 until the panel maps). Expect: left/middle click open the
# panel focused; clicking the icon again, another window or the top bar closes it; right click = menu.
. /tmp/session.env
X=$1
st() { w=$(xdotool search --name "Quota by Exodus" 2>/dev/null | head -1); if [ -n "$w" ]; then xwininfo -id "$w" | sed -n "s/.*Map State: //p"; else echo no-window; fi; }
a() { xdotool getactivewindow getwindowname 2>/dev/null; }
other() { xdotool mousemove 300 200 click 1; sleep 1.5; }
pgrep -x xmessage >/dev/null || (xmessage -geometry 700x500+50+100 -buttons "" "Another app window" >/dev/null 2>&1 &); sleep 1
other
for i in 1 2; do
  xdotool mousemove "$X" 12 click 1; sleep 1.5; echo "left click: $(st) active=$(a)"
  other; echo "click other window: $(st)"
done
xdotool mousemove "$X" 12 click 1; sleep 1.5; echo "left click: $(st)"
xdotool mousemove "$X" 12 click 1; sleep 1.5; echo "left click again: $(st)"
xdotool mousemove "$X" 12 click 1; sleep 1.5; echo "left click: $(st)"
xdotool mousemove 700 12 click 1; sleep 1.5; echo "click top bar: $(st)"; xdotool key Escape
xdotool mousemove "$X" 12 click 2; sleep 1.5; echo "middle click: $(st) active=$(a)"
other; echo "click other window: $(st)"
xdotool mousemove "$X" 12 click 3; sleep 1.5; echo "right click: $(st) (menu)"; xdotool key Escape
echo "panics: $(grep -ic panic /tmp/app.log)"
