// Settings → Alerts: at what share of the budget used the tray ring, the budget bar and the ccline
// status line turn to the warning color, then to the critical color.
//
// One track shows the three zones (normal, warning, critical). Its two thumbs follow the pointer 1:1
// from where they were grabbed, resist softly at their limits and settle back on a critically damped
// spring; arrow keys give exact values. Each level's row reveals its colors on demand. app.js owns
// saving and re-rendering; `AlertControls.held()` keeps it from rebuilding the view under the hand.

const ALERT_SWATCHES = [
  { color: "#ffd60a", name: "Yellow", light: true },
  { color: "#ff9f0a", name: "Orange", light: true },
  { color: "#ff453a", name: "Red" },
  { color: "#ff375f", name: "Pink" },
  { color: "#bf5af2", name: "Purple" },
];
const ALERT_DEFAULTS = { warning_pct: 75, warning_color: "#ffd60a", critical_pct: 90, critical_color: "#ff453a" };
const ALERT_LEVELS = [
  { key: "warning", label: "Warning" },
  { key: "critical", label: "Critical" },
];

Views.alerts = function (state, { error, openColor }) {
  const a = state.alerts || ALERT_DEFAULTS;
  const b = state.cache && state.cache.snapshot && state.cache.snapshot.budget;
  const used = b && b.max_budget > 0 ? Math.min(1, Math.max(0, b.spend / b.max_budget)) * 100 : null;
  const now =
    used === null
      ? ""
      : `<span class="threshold-now" style="--at:${used}" aria-hidden="true"></span>`;
  const nowLabel =
    used === null ? "" : `<span class="alerts-now">Now at ${Math.round(used)}%</span>`;

  const thumb = ({ key, label }) => {
    const pct = a[`${key}_pct`];
    const [min, max] = key === "warning" ? [1, a.critical_pct - 1] : [a.warning_pct + 1, 100];
    return `<span class="threshold-thumb" id="thumb-${key}" data-level="${key}" role="slider" tabindex="0"
      aria-label="${label} level" aria-valuemin="${min}" aria-valuemax="${max}" aria-valuenow="${pct}"
      aria-valuetext="${label} at ${pct}% used" style="--pos:${pct}"><span class="thumb-core"></span></span>`;
  };

  const row = ({ key, label }) => {
    const color = a[`${key}_color`];
    const open = openColor === key;
    const swatches = ALERT_SWATCHES.map(
      (s, i) => `<button class="swatch ${s.color === color ? "selected" : ""} ${s.light ? "light" : ""}" id="swatch-${key}-${i}"
        data-action="alert-color" data-level="${key}" data-color="${s.color}" style="--swatch:${s.color}"
        role="radio" aria-checked="${s.color === color}" aria-label="${s.name}"></button>`,
    ).join("");
    return `
      <div class="alert-level ${open ? "open" : ""}" data-level="${key}">
        <button class="alert-row" id="alert-row-${key}" data-action="alert-disclose" data-level="${key}" aria-expanded="${open}" aria-controls="colors-${key}">
          <span class="alert-chip" style="background:${esc(color)}"></span>
          <span class="alert-name">${label}</span>
          <span class="alert-value num"><span data-value="${key}">${a[`${key}_pct`]}</span>%</span>
          <span class="alert-chevron">${Icons.chevron}</span>
        </button>
        <div class="alert-colors" id="colors-${key}">
          <div class="alert-colors-inner">
            <div class="swatches" role="radiogroup" aria-label="${label} color">${swatches}</div>
          </div>
        </div>
      </div>`;
  };

  return `
    <div class="card alerts-card">
      <div class="alerts-head"><span class="metric-label">Alerts</span>${nowLabel}</div>
      <div class="threshold" style="--w:${a.warning_pct};--c:${a.critical_pct};--warning-color:${esc(a.warning_color)};--critical-color:${esc(a.critical_color)}">
        <div class="threshold-rail"><span class="threshold-zones"></span>${now}</div>
        ${ALERT_LEVELS.map(thumb).join("")}
      </div>
      <div class="alert-rows">${ALERT_LEVELS.map(row).join("")}</div>
    </div>
    <div class="card-footnote ${error ? "notice" : ""}">${error ? esc(error) : "Colors the tray icon, the budget bar and the Claude Code status line."}</div>`;
};

// --- Interaction -------------------------------------------------------------------------------

const AlertControls = {
  // Critically damped spring (no overshoot: thumbs are placed, not flicked), response in seconds.
  RESPONSE: 0.3,
  values: null, // { warning, critical } while the settings view is up: the committed (whole) values
  drag: null,
  springs: {},
  holdUntil: 0,
  retry: null,
  timer: null,
  keyTimer: null,

  attach(root, { save, fit, disclosed }) {
    this.root = root;
    this.save = save;
    this.fit = fit;
    this.disclosed = disclosed;
    root.addEventListener("pointerdown", (e) => this.down(e));
    root.addEventListener("pointermove", (e) => this.move(e));
    root.addEventListener("pointerup", (e) => this.up(e));
    root.addEventListener("pointercancel", (e) => this.up(e));
    root.addEventListener("keydown", (e) => this.key(e));
  },

  // --- Holding back re-renders (a drag in progress, or a change still animating) ---

  busy() {
    return Boolean(this.drag) || this.keyTimer !== null || performance.now() < this.holdUntil;
  },

  // True when app.js should skip this render; it's retried once the controls are idle.
  held(retry) {
    if (!this.busy()) return false;
    this.retry = retry;
    this.schedule();
    return true;
  },

  hold(ms) {
    this.holdUntil = Math.max(this.holdUntil, performance.now() + ms);
    this.schedule();
  },

  schedule() {
    clearTimeout(this.timer);
    if (!this.retry || this.drag || this.keyTimer !== null) return;
    this.timer = setTimeout(() => {
      const retry = this.retry;
      this.retry = null;
      if (retry) retry();
    }, Math.max(0, this.holdUntil - performance.now()));
  },

  // --- Track geometry ---

  track() {
    return this.root.querySelector(".threshold");
  },

  read() {
    const t = this.track();
    return { warning: Number(t.style.getPropertyValue("--w")), critical: Number(t.style.getPropertyValue("--c")) };
  },

  bounds(level, v) {
    return level === "warning" ? [1, v.critical - 1] : [v.warning + 1, 100];
  },

  // Pointer x → percent of the track (0–100, unclamped).
  pctAt(clientX) {
    const rail = this.track().querySelector(".threshold-rail").getBoundingClientRect();
    return ((clientX - rail.left) / rail.width) * 100;
  },

  railWidth() {
    return this.track().querySelector(".threshold-rail").getBoundingClientRect().width;
  },

  // Draws a thumb at `visual` (may sit past its limit while rubber-banding) and its zone at the
  // clamped whole value; updates the row's number and the slider's ARIA values.
  paint(level, visual) {
    const t = this.track();
    const v = this.values;
    const [min, max] = this.bounds(level, v);
    const value = Math.round(Math.min(max, Math.max(min, visual)));
    v[level] = value;
    const thumb = t.querySelector(`#thumb-${level}`);
    thumb.style.setProperty("--pos", visual);
    t.style.setProperty(level === "warning" ? "--w" : "--c", Math.min(max, Math.max(min, visual)));
    thumb.setAttribute("aria-valuenow", value);
    thumb.setAttribute("aria-valuetext", `${level === "warning" ? "Warning" : "Critical"} at ${value}% used`);
    const other = level === "warning" ? "critical" : "warning";
    const [omin, omax] = this.bounds(other, v);
    const otherThumb = t.querySelector(`#thumb-${other}`);
    otherThumb.setAttribute("aria-valuemin", omin);
    otherThumb.setAttribute("aria-valuemax", omax);
    const label = this.root.querySelector(`[data-value="${level}"]`);
    if (label) label.textContent = value;
  },

  // --- Springs ---

  // Critically damped: x(t) = target + (x0 + (v0 + ωx0)t)e^(−ωt), started from the on-screen value
  // and the hand's velocity, so a release or a tap on the rail never jumps.
  springTo(level, from, target, velocity = 0, done) {
    this.stop(level);
    if (Motion.reduced()) {
      this.paint(level, target);
      if (done) done();
      return;
    }
    const w = (2 * Math.PI) / this.RESPONSE;
    const x0 = from - target;
    const start = performance.now();
    const tick = (now) => {
      const t = (now - start) / 1000;
      const x = (x0 + (velocity + w * x0) * t) * Math.exp(-w * t);
      if (Math.abs(x) < 0.02 && t > 0.05) {
        this.paint(level, target);
        delete this.springs[level];
        if (done) done();
        return;
      }
      this.paint(level, target + x);
      this.springs[level] = requestAnimationFrame(tick);
    };
    this.springs[level] = requestAnimationFrame(tick);
  },

  stop(level) {
    if (this.springs[level]) cancelAnimationFrame(this.springs[level]);
    delete this.springs[level];
  },

  // On-screen position of a thumb (mid-spring, it differs from its value).
  shown(level) {
    return Number(this.track().querySelector(`#thumb-${level}`).style.getPropertyValue("--pos"));
  },

  // --- Pointer ---

  down(e) {
    const track = e.target.closest(".threshold");
    if (!track || e.button !== 0) return;
    e.preventDefault();
    this.values = this.read();
    const at = this.pctAt(e.clientX);
    const px = this.railWidth() / 100;
    const near = ALERT_LEVELS.map(({ key }) => ({ key, gap: Math.abs(this.shown(key) - at) * px })).sort((x, y) => x.gap - y.gap);
    const onThumb = e.target.closest(".threshold-thumb");
    let level = onThumb ? onThumb.dataset.level : near[0].key;
    // Stacked thumbs: wait for the first move to tell which one the hand means (left = warning).
    const undecided = Math.abs(near[0].gap - near[1].gap) < 6 && near[0].gap < 14;
    let offset = onThumb ? this.shown(level) - at : 0;
    let tap = null;
    if (!onThumb && !undecided) {
      // A tap on the rail brings the nearer thumb there on a spring, then the drag takes over.
      const [min, max] = this.bounds(level, this.values);
      tap = Math.round(Math.min(max, Math.max(min, at)));
      this.springTo(level, this.shown(level), tap);
    }
    if (undecided) {
      level = null;
      offset = 0;
    }
    track.setPointerCapture(e.pointerId);
    this.drag = { id: e.pointerId, level, offset, tap, startX: e.clientX, samples: [{ x: at, t: e.timeStamp }], moved: false };
    if (level) this.track().querySelector(`#thumb-${level}`).classList.add("pressed");
    this.root.querySelector(`#thumb-${level || near[0].key}`).focus({ preventScroll: true });
  },

  move(e) {
    const d = this.drag;
    if (!d || e.pointerId !== d.id) return;
    const at = this.pctAt(e.clientX);
    if (!d.level) {
      if (Math.abs(e.clientX - d.startX) < 2) return;
      d.level = e.clientX < d.startX ? "warning" : "critical";
      d.offset = this.shown(d.level) - this.pctAt(d.startX);
      this.track().querySelector(`#thumb-${d.level}`).classList.add("pressed");
      this.root.querySelector(`#thumb-${d.level}`).focus({ preventScroll: true });
    }
    this.stop(d.level);
    d.moved = true;
    d.samples.push({ x: at, t: e.timeStamp });
    if (d.samples.length > 5) d.samples.shift();
    const want = at + d.offset;
    const [min, max] = this.bounds(d.level, this.values);
    const px = this.railWidth() / 100;
    // Past a limit the thumb follows less and less (rubber band) instead of stopping dead.
    const band = (over) => rubberband(over * px, 24) / px;
    const visual = want < min ? min - band(min - want) : want > max ? max + band(want - max) : want;
    this.paint(d.level, visual);
  },

  up(e) {
    const d = this.drag;
    if (!d || e.pointerId !== d.id) return;
    this.drag = null;
    this.track()?.querySelectorAll(".threshold-thumb").forEach((t) => t.classList.remove("pressed"));
    if (!d.level) return this.schedule();
    const level = d.level;
    const s = d.samples;
    const dt = s.length > 1 ? (s[s.length - 1].t - s[0].t) / 1000 : 0;
    const velocity = dt > 0 ? (s[s.length - 1].x - s[0].x) / dt : 0;
    this.hold(this.RESPONSE * 1000 + 150);
    if (!d.moved && d.tap !== null) {
      // A plain tap: its spring is still on the way; save where it lands.
      return this.commit({ ...this.values, [level]: d.tap });
    }
    // Settle on the whole value (and back inside the limits) carrying the hand's speed.
    this.springTo(level, this.shown(level), this.values[level], velocity);
    this.commit({ ...this.values });
  },

  // --- Keyboard: arrows ±1, Shift+arrows or Page Up/Down ±10, Home/End to the limits ---

  key(e) {
    const thumb = e.target.closest && e.target.closest(".threshold-thumb");
    if (!thumb) return;
    const level = thumb.dataset.level;
    this.values = this.values || this.read();
    const [min, max] = this.bounds(level, this.values);
    const now = this.values[level];
    const step = e.shiftKey ? 10 : 1;
    const next = {
      ArrowLeft: now - step,
      ArrowDown: now - step,
      ArrowRight: now + step,
      ArrowUp: now + step,
      PageDown: now - 10,
      PageUp: now + 10,
      Home: min,
      End: max,
    }[e.key];
    if (next === undefined) return;
    e.preventDefault();
    this.stop(level);
    this.paint(level, Math.min(max, Math.max(min, next)));
    // Save once the keys go quiet, not on every press.
    clearTimeout(this.keyTimer);
    this.keyTimer = setTimeout(() => {
      this.keyTimer = null;
      this.hold(150);
      this.commit({ ...this.values });
    }, 400);
  },

  commit(v) {
    const saved = this.saved;
    if (saved && saved.warning === v.warning && saved.critical === v.critical) return this.schedule();
    this.saved = { ...v };
    this.save({ warning_pct: v.warning, critical_pct: v.critical });
  },

  // Called by app.js after every render of the settings view.
  mounted(state) {
    const a = state.alerts || ALERT_DEFAULTS;
    this.values = { warning: a.warning_pct, critical: a.critical_pct };
    this.saved = { ...this.values };
    this.springs = {};
  },

  // --- Colors ---

  // Reveal one level's colors (and close the other). The window grows before the reveal and shrinks
  // after the close, so nothing gets clipped.
  disclose(el) {
    const level = el.closest(".alert-level");
    const opening = !level.classList.contains("open");
    const others = [...this.root.querySelectorAll(".alert-level.open")].filter((l) => l !== level);
    const height = (l) => l.querySelector(".alert-colors-inner").scrollHeight;
    const delta = (opening ? height(level) : -height(level)) - others.reduce((s, l) => s + height(l), 0);
    const apply = () => {
      others.forEach((l) => this.setOpen(l, false));
      this.setOpen(level, opening);
    };
    this.disclosed(opening ? level.dataset.level : null);
    this.hold(360);
    if (delta > 0) {
      this.fit(delta);
      requestAnimationFrame(apply);
    } else {
      apply();
      setTimeout(() => this.fit(0), 340);
    }
  },

  setOpen(level, open) {
    level.classList.toggle("open", open);
    level.querySelector(".alert-row").setAttribute("aria-expanded", open);
  },

  // Show the new color at once (chip, thumb, zone, selection), then save.
  pick(el) {
    const level = el.dataset.level;
    const color = el.dataset.color;
    el.closest(".swatches").querySelectorAll(".swatch").forEach((s) => {
      const on = s === el;
      s.classList.toggle("selected", on);
      s.setAttribute("aria-checked", on);
    });
    el.closest(".alert-level").querySelector(".alert-chip").style.background = color;
    this.track().style.setProperty(`--${level}-color`, color);
    this.hold(260);
    this.save({ [`${level}_color`]: color });
  },
};

// The further past the limit, the less the thumb follows (Apple's rubber band).
function rubberband(overshoot, dimension, constant = 0.55) {
  return (overshoot * dimension * constant) / (dimension + constant * Math.abs(overshoot));
}
