// Panel controller: holds the current view, talks to the Rust side, re-renders on events.

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
let period = loadPeriod();

// The chosen Cost period is a per-viewer convenience; storage may be unavailable.
function loadPeriod() {
  try {
    return localStorage.getItem("period") || "today";
  } catch {
    return "today";
  }
}

function render() {
  if (!state) return;
  // Don't redraw the sign-in form under the user's cursor.
  if (document.activeElement && document.activeElement.id === "proxy-url" && !state.signed_in && !login) return;
  if (login && login.phase === "waiting") app.innerHTML = Views.waiting(login);
  else if (login && login.phase === "team") app.innerHTML = Views.pickTeam(login.teams);
  else if (!state.signed_in) app.innerHTML = Views.signIn(state, signInError, draftUrl);
  else app.innerHTML = Views.usage(state, period);
  requestAnimationFrame(() => invoke("fit_height", { height: document.body.scrollHeight }));
}

async function act(action, el) {
  switch (action) {
    case "refresh": {
      el.disabled = true;
      el.classList.add("busy");
      state = await invoke("refresh");
      break;
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
    case "period":
      period = el.dataset.period;
      try {
        localStorage.setItem("period", period);
      } catch {}
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
    case "sign-out":
      state = await invoke("sign_out");
      break;
    case "quit":
      await invoke("quit");
      return;
  }
  render();
}

app.addEventListener("click", (e) => {
  const el = e.target.closest("[data-action]");
  if (el) act(el.dataset.action, el).catch((err) => console.error(err));
});

app.addEventListener("input", (e) => {
  if (e.target.id === "proxy-url") draftUrl = e.target.value;
});

app.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && e.target.id === "proxy-url") act("sign-in", app.querySelector('[data-action="sign-in"]'));
});

listen("state", (e) => {
  state = e.payload;
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

// Keep relative times ("Updated 2m ago") fresh while the panel is open.
setInterval(() => {
  if (state) {
    state.now = Math.floor(Date.now() / 1000);
    if (!login && state.signed_in) render();
  }
}, 30000);

invoke("get_state").then((s) => {
  state = s;
  render();
});
