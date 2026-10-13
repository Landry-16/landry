// Landry for GNOME Shell. Under Wayland only the Shell can list windows, focus one or watch the
// clipboard: this extension does it on D-Bus (dev.landry.Shell at /dev/landry/Shell), for Landry only.

import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const IFACE = `<node><interface name="dev.landry.Shell">
  <method name="Windows"><arg type="s" direction="out" name="json_base64"/></method>
  <method name="Activate"><arg type="u" direction="in" name="id"/><arg type="b" direction="out" name="found"/></method>
  <method name="Close"><arg type="u" direction="in" name="id"/><arg type="b" direction="out" name="found"/></method>
  <method name="Clipboard"><arg type="s" direction="out" name="json_base64"/></method>
  <method name="ClipboardRemove"><arg type="u" direction="in" name="id"/></method>
  <method name="ClipboardClear"/>
  <method name="SetPaused"><arg type="b" direction="in" name="paused"/></method>
</interface></node>`;

const MAX_ITEMS = 100;
const MAX_LENGTH = 100000;

// Base64 keeps the reply free of GVariant string escaping.
const encode = (value) => GLib.base64_encode(new TextEncoder().encode(JSON.stringify(value)));

function exeOf(pid) {
    try {
        return GLib.file_read_link(`/proc/${pid}/exe`);
    } catch {
        return '';
    }
}

function parentOf(pid) {
    try {
        const [, data] = GLib.file_get_contents(`/proc/${pid}/stat`);
        const stat = new TextDecoder().decode(data);
        return parseInt(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1]);
    } catch {
        return 0;
    }
}

// Landry calls through gdbus: accept Landry itself or a program it started. "landry (deleted)" is
// a Landry still running after an update replaced its binary.
const isLandry = (pid) => [pid, parentOf(pid)].some((p) => GLib.path_get_basename(exeOf(p)).startsWith('landry'));

export default class LandryShell extends Extension {
    enable() {
        this._history = [];
        this._next = 1;
        this._paused = false;
        this._clipboard = St.Clipboard.get_default();
        this._selection = global.display.get_selection();
        this._ownerId = this._selection.connect('owner-changed', (_s, type, source) => this._onOwnerChanged(type, source));
        this._dbus = Gio.DBusExportedObject.wrapJSObject(IFACE, this);
        this._dbus.export(Gio.DBus.session, '/dev/landry/Shell');
    }

    disable() {
        this._dbus?.unexport();
        this._dbus = null;
        if (this._ownerId)
            this._selection.disconnect(this._ownerId);
        this._ownerId = 0;
        this._history = [];
    }

    _onOwnerChanged(type, source) {
        if (type !== Meta.SelectionType.SELECTION_CLIPBOARD || this._paused || !source)
            return;
        // Password managers mark what they copy: never keep it.
        if (source.get_mimetypes().includes('x-kde-passwordManagerHint'))
            return;
        this._clipboard.get_text(St.ClipboardType.CLIPBOARD, (_c, text) => {
            if (!text || !text.trim() || text.length > MAX_LENGTH)
                return;
            this._history = this._history.filter((h) => h.text !== text);
            this._history.unshift({id: this._next++, text, time: Math.floor(Date.now() / 1000)});
            this._history.length = Math.min(this._history.length, MAX_ITEMS);
        });
    }

    _windows() {
        const tracker = Shell.WindowTracker.get_default();
        // Most recently used first.
        return global.display.get_tab_list(Meta.TabList.NORMAL_ALL, null)
            .filter((w) => !w.is_skip_taskbar())
            .map((w) => {
                const app = tracker.get_window_app(w);
                return {
                    id: w.get_id(),
                    title: w.get_title() ?? '',
                    app: app?.get_id() ?? '',
                    name: app?.get_name() ?? w.get_wm_class() ?? '',
                    workspace: w.get_workspace()?.index() ?? -1,
                    focused: w.has_focus(),
                    minimized: w.minimized,
                };
            });
    }

    _find(id) {
        return global.display.list_all_windows().find((w) => w.get_id() === id);
    }

    // Answer only Landry: check the caller's executable before running `reply`.
    _guard(invocation, reply) {
        Gio.DBus.session.call(
            'org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'GetConnectionUnixProcessID',
            new GLib.Variant('(s)', [invocation.get_sender()]), new GLib.VariantType('(u)'),
            Gio.DBusCallFlags.NONE, -1, null,
            (connection, result) => {
                try {
                    const [pid] = connection.call_finish(result).deepUnpack();
                    if (!isLandry(pid))
                        throw new Error('réservé à Landry');
                    invocation.return_value(reply());
                } catch (e) {
                    invocation.return_error_literal(Gio.DBusError, Gio.DBusError.ACCESS_DENIED, e.message);
                }
            });
    }

    WindowsAsync(_params, invocation) {
        this._guard(invocation, () => new GLib.Variant('(s)', [encode(this._windows())]));
    }

    ActivateAsync([id], invocation) {
        this._guard(invocation, () => {
            const w = this._find(id);
            if (w)
                Main.activateWindow(w);
            return new GLib.Variant('(b)', [!!w]);
        });
    }

    CloseAsync([id], invocation) {
        this._guard(invocation, () => {
            const w = this._find(id);
            w?.delete(global.get_current_time());
            return new GLib.Variant('(b)', [!!w]);
        });
    }

    ClipboardAsync(_params, invocation) {
        this._guard(invocation, () => new GLib.Variant('(s)', [encode({paused: this._paused, items: this._history})]));
    }

    ClipboardRemoveAsync([id], invocation) {
        this._guard(invocation, () => {
            this._history = this._history.filter((h) => h.id !== id);
            return null;
        });
    }

    ClipboardClearAsync(_params, invocation) {
        this._guard(invocation, () => {
            this._history = [];
            return null;
        });
    }

    SetPausedAsync([paused], invocation) {
        this._guard(invocation, () => {
            this._paused = paused;
            return null;
        });
    }
}
