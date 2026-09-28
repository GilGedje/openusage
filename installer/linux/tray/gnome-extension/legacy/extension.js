/* Quota by Exodus.Ai — tray click (GNOME Shell 3.36–44: Ubuntu 20.04 and 22.04).
 *
 * Ubuntu shows tray icons through its AppIndicator extension, which opens the icon's menu on a
 * single left click and only asks the app to open itself on a double click. This makes a single left
 * click on Quota's icon open its panel instead (right click still shows the menu). It doesn't touch
 * any other icon, and draws nothing of its own.
 *
 * Kept in step with ../modern/extension.js (the same logic in GNOME 45+'s module format).
 */

const { GLib, Clutter } = imports.gi;
const Main = imports.ui.main;

// The tray icon's StatusNotifierItem id (crates/sni-tray).
const APP_ID = 'litellm-usage';

let timer = 0;
let hooked = [];

// Opens the panel under the icon: asks the app to activate at the icon's bottom centre.
function activate(button, event) {
    const menu = Main.panel.menuManager.activeMenu;
    if (menu)
        menu.close();
    const [bx, by] = button.get_transformed_position();
    const [bw, bh] = button.get_transformed_size();
    const x = Math.round(bx + bw / 2);
    const y = Math.round(by + bh);
    const indicator = button._indicator;
    if (indicator.open.length >= 2)
        indicator.open(x, y, event.get_time());
    else if (indicator._proxy && indicator._proxy.ActivateRemote)
        indicator._proxy.ActivateRemote(x, y);
    else
        indicator.open();
}

// Hooks Quota's icon once it's in the top bar (icons come and go as apps start and quit).
function hook() {
    for (const [role, button] of Object.entries(Main.panel.statusArea)) {
        // AppIndicator icons sit under 'appindicator-…'; 'in' avoids warnings for its other icons.
        const indicator = role.startsWith('appindicator-') && button && button._indicator;
        if (!indicator || !('id' in indicator) || indicator.id !== APP_ID)
            continue;
        if (hooked.some(h => h.button === button))
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
        const entry = { button, id };
        button.connect('destroy', () => { hooked = hooked.filter(h => h !== entry); });
        hooked.push(entry);
    }
}

function init() {}

function enable() {
    hook();
    timer = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 2, () => {
        hook();
        return GLib.SOURCE_CONTINUE;
    });
}

function disable() {
    if (timer)
        GLib.source_remove(timer);
    timer = 0;
    for (const { button, id } of hooked)
        button.disconnect(id);
    hooked = [];
}
