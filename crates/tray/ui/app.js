// Panel controller: holds the current view, talks to the Rust side, re-renders on events.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const app = document.getElementById("app");

let state = null;
// null | { phase: "waiting", code, link } | { phase: "team", teams }
let login = null;
let signInError = null;
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
  if (login && login.phase === "waiting") app.innerHTML = Views.waiting(login);
  else if (login && login.phase === "team") app.innerHTML = Views.pickTeam(login.teams);
  else if (!state.signed_in) app.innerHTML = Views.signIn(state, signInError);
  else app.innerHTML = Views.usage(state, period);
  requestAnimationFrame(() => invoke("fit_height", { height: document.body.scrollHeight }));
}

async function act(action, el) {
  switch (action) {
    case "refresh": {
      el.classList.add("spinning");
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
