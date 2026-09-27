// Panel controller: holds the current view, talks to the Rust side, re-renders on events, and runs
// the motion (critically damped, interruptible, reduced-motion aware — see Motion below).

// Tauri 2 exposes invoke under `core`, Tauri 1 (the Ubuntu 20.04 build) under `tauri`.
const { invoke } = window.__TAURI__.core || window.__TAURI__.tauri;
const { listen } = window.__TAURI__.event;

const app = document.getElementById("app");

let state = null;
// null | { phase: "waiting", code, link } | { phase: "team", teams }
let login = null;
let signInError = null;
// What the user has typed in the sign-in URL field, so background updates don't wipe it.
let draftUrl = null;
let settings = null; // null, or { error, draftUrl } while the settings view is open
let period = stored("period", "today");
let theme = stored("theme", "system");
let shownView = null;

// Per-viewer conveniences; storage may be unavailable.
function stored(key, fallback) {
  try {
    return localStorage.getItem(key) || fallback;
  } catch {
    return fallback;
  }
}
function store(key, value) {
  try {
    localStorage.setItem(key, value);
  } catch {}
}

// --- Motion ------------------------------------------------------------------------------------
// Critically damped curves (no overshoot: nothing here is flicked), transform/opacity only, and a
// plain short cross-fade when the user prefers reduced motion.
const Motion = {
  ease: "cubic-bezier(0.2, 0.8, 0.2, 1)",
  reduced: () => matchMedia("(prefers-reduced-motion: reduce)").matches,

  // Settings slides in from the right (where its button is) and back out to the right.
  enter(el, kind) {
    if (!kind) return;
    if (this.reduced()) {
      el.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 150, easing: "ease-out" });
      return;
    }
    const from = { push: "translateX(28px)", pop: "translateX(-28px)", fade: "translateY(6px)" }[kind];
    el.animate([{ opacity: 0, transform: from }, { opacity: 1, transform: "none" }], {
      duration: kind === "fade" ? 220 : 300,
      easing: this.ease,
    });
  },

  // A segmented control's thumb slides from the old option to the new one.
  slideThumb(control, fromIndex) {
    if (!control || fromIndex == null) return;
    const to = control.style.getPropertyValue("--segment-index");
    if (String(fromIndex) === to || this.reduced()) return;
    control.classList.add("no-thumb-transition");
    control.style.setProperty("--segment-index", fromIndex);
    void control.offsetWidth; // commit the start position
    control.classList.remove("no-thumb-transition");
    control.style.setProperty("--segment-index", to);
  },

  // The donut's arcs draw in one after another; the daily bars grow from the baseline.
  drawCost(root) {
    if (this.reduced()) return;
    root.querySelectorAll(".arc").forEach((arc, i) => {
      const dash = Number(arc.dataset.dash);
      const circ = Number(arc.dataset.circ);
      arc.animate([{ strokeDasharray: `0 ${circ}` }, { strokeDasharray: `${dash} ${circ - dash}` }], {
        duration: 520,
        delay: i * 45,
        easing: this.ease,
        fill: "backwards",
      });
    });
    root.querySelector(".chart")?.classList.add("grow");
  },

  toast(text) {
    document.querySelector(".toast")?.remove();
    const toast = document.createElement("div");
    toast.className = "toast";
    toast.textContent = text;
    document.body.appendChild(toast);
    const path = this.reduced()
      ? [{ opacity: 0 }, { opacity: 1 }]
      : [{ opacity: 0, transform: "translate(-50%, 12px)" }, { opacity: 1, transform: "translate(-50%, 0)" }];
    toast.animate(path, { duration: 240, easing: this.ease, fill: "both" });
    setTimeout(() => {
      // Leaves the way it came.
      toast.animate([...path].reverse(), { duration: 200, easing: "ease-in", fill: "both" }).finished.then(() => toast.remove());
    }, 3600);
  },
};

// --- Theme -------------------------------------------------------------------------------------
function applyTheme(pref, animate) {
  const root = document.documentElement;
  if (animate && !Motion.reduced()) {
    // Ease the light/dark change instead of a hard brightness jump.
    root.classList.add("theme-anim");
    setTimeout(() => root.classList.remove("theme-anim"), 320);
  }
  if (pref === "system") delete root.dataset.theme;
  else root.dataset.theme = pref;
}
applyTheme(theme, false);

// --- Rendering ---------------------------------------------------------------------------------
function viewName() {
  if (settings) return "settings";
  if (login && login.phase === "waiting") return "waiting";
  if (login && login.phase === "team") return "team";
  if (!state.signed_in) return "signin";
  return "usage";
}

// `hint` marks what just happened: "period" (switched Today/Yesterday/30 Days), "theme", or null.
function render(hint = null, prev = {}) {
  if (!state) return;
  // Don't redraw a form under the user's cursor.
  const typing = document.activeElement && document.activeElement.classList.contains("text-input");
  const name = viewName();
  if (typing && name === shownView && (name === "settings" || name === "signin")) return;

  const html = {
    settings: () => Views.settings(state, { ...settings, theme }),
    waiting: () => Views.waiting(login),
    team: () => Views.pickTeam(login.teams),
    signin: () => Views.signIn(state, signInError, draftUrl),
    usage: () => Views.usage(state, period),
  }[name]();
  app.innerHTML = `<div class="view">${html}</div>`;
  const view = app.firstElementChild;

  let transition = null;
  if (shownView !== null && name !== shownView) {
    transition = name === "settings" ? "push" : shownView === "settings" ? "pop" : "fade";
  }
  Motion.enter(view, transition);
  if (name === "usage" && (hint === "period" || (transition && transition !== "pop") || shownView === null)) {
    Motion.drawCost(view);
  }
  if (hint === "period") Motion.slideThumb(view.querySelector(".cost-card .segmented"), prev.index);
  if (hint === "theme") Motion.slideThumb(view.querySelector(".segmented"), prev.index);
  shownView = name;
  requestAnimationFrame(() => invoke("fit_height", { height: document.body.scrollHeight }));
}

const PERIOD_KEYS = ["today", "yesterday", "30d"];
const THEME_KEYS = ["system", "light", "dark"];

async function act(action, el) {
  switch (action) {
    case "refresh": {
      el.disabled = true;
      el.classList.add("busy");
      const started = Date.now();
      state = await invoke("refresh");
      // Let one turn of the spinner finish so a fast refresh still reads as "done".
      await new Promise((r) => setTimeout(r, Math.max(0, 600 - (Date.now() - started))));
      break;
    }
    case "period": {
      const index = PERIOD_KEYS.indexOf(period);
      if (el.dataset.period === period) return;
      period = el.dataset.period;
      store("period", period);
      return render("period", { index });
    }
    case "share": {
      const snap = state && state.cache && state.cache.snapshot;
      if (!snap) return;
      el.disabled = true;
      try {
        const { copied } = await Share.exportCost(snap, period);
        Motion.toast(copied ? "Image copied and saved to Downloads" : "Image saved to Downloads");
      } catch (e) {
        Motion.toast(`Couldn't export: ${e}`);
      }
      el.disabled = false;
      return;
    }
    case "sign-in": {
      const url = document.getElementById("proxy-url").value.trim();
      el.disabled = true;
      signInError = null;
      try {
        const started = await invoke("start_login", { url: url || null });
        login = { phase: "waiting", code: started.code, link: started.link };
        draftUrl = null;
      } catch (e) {
        signInError = String(e);
      }
      break;
    }
    case "settings":
      settings = { error: null, draftUrl: null };
      break;
    case "close-settings":
      settings = null;
      break;
    case "save-url": {
      el.disabled = true;
      const url = document.getElementById("set-url").value.trim();
      try {
        state = await invoke("save_settings", { url });
        settings = null;
      } catch (e) {
        settings = { error: String(e), draftUrl: url };
      }
      break;
    }
    case "theme": {
      const index = THEME_KEYS.indexOf(theme);
      theme = el.dataset.theme;
      store("theme", theme);
      applyTheme(theme, true);
      return render("theme", { index });
    }
    case "sign-out":
      state = await invoke("sign_out");
      settings = null;
      break;
    case "open-usage":
      if (state && state.usage_url) await invoke("open_url", { url: state.usage_url });
      return;
    case "open-status":
      if (state && state.status_url) await invoke("open_url", { url: state.status_url });
      return;
    case "open-link":
      if (login && login.link) await invoke("open_url", { url: login.link });
      return;
    case "team":
      await invoke("choose_team", { teamId: el.dataset.team });
      login = { ...login, phase: "waiting" };
      break;
    case "cancel-login":
      await invoke("cancel_login");
      login = null;
      break;
  }
  render();
}

app.addEventListener("click", (e) => {
  const el = e.target.closest("[data-action]");
  if (el && !el.disabled) act(el.dataset.action, el).catch((err) => console.error(err));
});

app.addEventListener("input", (e) => {
  if (e.target.id === "proxy-url") draftUrl = e.target.value;
});

app.addEventListener("keydown", (e) => {
  if (e.key !== "Enter") return;
  if (e.target.id === "proxy-url") act("sign-in", app.querySelector('[data-action="sign-in"]'));
  if (e.target.id === "set-url") act("save-url", app.querySelector('[data-action="save-url"]'));
});

listen("state", (e) => {
  state = e.payload;
  render();
});

listen("show-settings", () => {
  settings = { error: null, draftUrl: null };
  render();
});

listen("login", (e) => {
  const ev = e.payload;
  if (ev.status === "select-team") login = { ...login, phase: "team", teams: ev.teams };
  else if (ev.status === "done") login = null;
  else if (ev.status === "error") {
    login = null;
    signInError = ev.message;
  }
  render();
});

// The footer countdown ticks every second without redrawing the panel.
setInterval(() => {
  const el = document.getElementById("countdown");
  if (el && state) el.textContent = Format.countdown(state);
}, 1000);

invoke("get_state").then((s) => {
  state = s;
  render();
});
