/* Quota by Exodus.Ai — tray click (GNOME Shell 45+: Ubuntu 24.04 and 26.04).
 *
 * Ubuntu shows tray icons through its AppIndicator extension, which opens the icon's menu on a
 * single left click and only asks the app to open itself on a double click. This makes a single left
 * click on Quota's icon open its panel instead (right click still shows the menu). It doesn't touch
 * any other icon, and draws nothing of its own.
 *
 * Kept in step with ../legacy/extension.js (the same logic in GNOME 3.36–44's format).
 */

import GLib from 'gi://GLib';
import Clutter from 'gi://Clutter';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

// The tray icon's StatusNotifierItem id (crates/sni-tray).
const APP_ID = 'litellm-usage';

// Opens the panel under the icon: asks the app to activate at the icon's bottom centre.
function activate(button, event) {
    Main.panel.menuManager.activeMenu?.close();
    const [bx, by] = button.get_transformed_position();
    const [bw, bh] = button.get_transformed_size();
    const x = Math.round(bx + bw / 2);
    const y = Math.round(by + bh);
    const result = button._indicator.open(x, y, event.get_time());
    result?.catch?.(logError);
}

export default class QuotaTrayClick extends Extension {
    enable() {
        this._hooked = [];
        this._hook();
        this._timer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
            this._hook();
            return GLib.SOURCE_CONTINUE;
        });
    }

    disable() {
        if (this._timer)
            GLib.source_remove(this._timer);
        this._timer = 0;
        for (const {button, id} of this._hooked)
            button.disconnect(id);
        this._hooked = [];
    }

    // Hooks Quota's icon once it's in the top bar (icons come and go as apps start and quit).
    _hook() {
        for (const [role, button] of Object.entries(Main.panel.statusArea)) {
            // AppIndicator icons sit under 'appindicator-…'; 'in' avoids warnings for its other icons.
            const indicator = role.startsWith('appindicator-') ? button?._indicator : null;
            if (!indicator || !('id' in indicator) || indicator.id !== APP_ID)
                continue;
            if (this._hooked.some(h => h.button === button))
                continue;
            // 'event' runs before the icon's own click handling, so stopping it here keeps the menu shut.
            // The panel opens on release: a window shown while the button is still held gets focus in a way
            // GTK doesn't register, so it would never hear that focus left (and wouldn't close on click-away).
            const id = button.connect('event', (actor, event) => {
                const type = event.type();
                if ((type !== Clutter.EventType.BUTTON_PRESS && type !== Clutter.EventType.BUTTON_RELEASE) ||
                    (event.get_button() !== Clutter.BUTTON_PRIMARY && event.get_button() !== Clutter.BUTTON_MIDDLE))
                    return Clutter.EVENT_PROPAGATE;
                if (type === Clutter.EventType.BUTTON_RELEASE)
                    activate(button, event);
                return Clutter.EVENT_STOP;
            });
            const entry = {button, id};
            button.connect('destroy', () => {
                this._hooked = this._hooked.filter(h => h !== entry);
            });
            this._hooked.push(entry);
        }
    }
}
