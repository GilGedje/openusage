// Exports the Cost card as a PNG to share: drawn on a canvas (no libraries — the app is air-gapped)
// with the current theme's colors, then saved to Downloads by the app and copied to the clipboard
// where the system allows it.

const Share = {
  WIDTH: 360,
  HEIGHT: 214,
  SCALE: 2,

  // Resolves a CSS color (including var(--token)) against the current theme.
  color(value) {
    const probe = document.createElement("span");
    probe.style.color = value;
    document.body.appendChild(probe);
    const resolved = getComputedStyle(probe).color;
    probe.remove();
    return resolved;
  },

  async exportCost(snap, period) {
    const { total, models } = Views.periodData(snap, period);
    const items = Views.legendItems(models);
    const label = PERIODS.find((p) => p.key === period).label;
    const canvas = this.draw(total.spend, items, label === "30 Days" ? "Last 30 Days" : label);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, "image/png"));
    const bytes = Array.from(new Uint8Array(await blob.arrayBuffer()));
    const date = new Date().toISOString().slice(0, 10);
    const path = await invoke("save_image", { bytes, name: `Claude cost - ${label} - ${date}` });
    let copied = false;
    try {
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
      copied = true;
    } catch {
      // Clipboard images aren't available everywhere (e.g. some Linux webviews); the file is enough.
    }
    return { path, copied };
  },

  draw(total, items, periodLabel) {
    const { WIDTH: W, HEIGHT: H, SCALE: S } = this;
    const canvas = document.createElement("canvas");
    canvas.width = W * S;
    canvas.height = H * S;
    const g = canvas.getContext("2d");
    g.scale(S, S);
    const font = getComputedStyle(document.body).fontFamily;
    const c = (v) => this.color(v);

    // Background: the panel's tray, a rounded card on it.
    g.fillStyle = c("var(--tray)");
    g.fillRect(0, 0, W, H);
    g.fillStyle = c("var(--card-fill)");
    this.roundRect(g, 12, 44, W - 24, H - 56, 12);
    g.fill();

    // Title: "Cost  Today" and the date.
    g.textBaseline = "alphabetic";
    g.fillStyle = c("var(--text)");
    g.font = `600 17px ${font}`;
    g.fillText("Cost", 16, 30);
    const titleWidth = g.measureText("Cost").width;
    g.fillStyle = c("var(--text-secondary)");
    g.font = `400 15px ${font}`;
    g.fillText(periodLabel, 16 + titleWidth + 8, 30);
    g.font = `400 11px ${font}`;
    const date = new Date().toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });
    g.fillText(date, W - 16 - g.measureText(date).width, 29);

    // Donut.
    const cx = 80, cy = 44 + (H - 56) / 2, r = 46, lw = 16;
    g.lineWidth = lw;
    if (!items.length || total <= 0) {
      g.strokeStyle = c("var(--meter-track)");
      g.beginPath();
      g.arc(cx, cy, r, 0, Math.PI * 2);
      g.stroke();
    } else {
      const gap = items.length > 1 ? 0.035 : 0;
      let start = -Math.PI / 2;
      for (const item of items) {
        const sweep = (item.spend / total) * Math.PI * 2;
        g.strokeStyle = c(item.color);
        g.beginPath();
        g.arc(cx, cy, r, start + gap / 2, start + Math.max(sweep - gap / 2, gap));
        g.stroke();
        start += sweep;
      }
    }
    g.textAlign = "center";
    g.fillStyle = c("var(--text)");
    g.font = `700 13px ${font}`;
    g.fillText(Format.money(total), cx, cy + 2);
    g.fillStyle = c("var(--text-secondary)");
    g.font = `400 10px ${font}`;
    g.fillText("dollars", cx, cy + 16);
    g.textAlign = "left";

    // Legend.
    const lx = 150, rows = items.length ? items : [{ name: "No spend", spend: 0, color: "var(--meter-track)" }];
    const rowH = 22;
    let y = cy - ((rows.length - 1) * rowH) / 2 + 4;
    for (const item of rows) {
      g.fillStyle = c(item.color);
      g.beginPath();
      g.arc(lx + 4, y - 4, 4.5, 0, Math.PI * 2);
      g.fill();
      g.fillStyle = c("var(--text)");
      g.font = `400 13px ${font}`;
      g.fillText(this.fit(g, item.name, 118), lx + 16, y);
      if (items.length) {
        g.fillStyle = c("var(--text-secondary)");
        g.font = `400 11px ${font}`;
        const amount = Format.money(item.spend);
        g.fillText(amount, W - 28 - g.measureText(amount).width, y);
      }
      y += rowH;
    }
    return canvas;
  },

  fit(g, text, width) {
    if (g.measureText(text).width <= width) return text;
    let t = text;
    while (t.length > 1 && g.measureText(t + "…").width > width) t = t.slice(0, -1);
    return t + "…";
  },

  roundRect(g, x, y, w, h, r) {
    g.beginPath();
    g.moveTo(x + r, y);
    g.arcTo(x + w, y, x + w, y + h, r);
    g.arcTo(x + w, y + h, x, y + h, r);
    g.arcTo(x, y + h, x, y, r);
    g.arcTo(x, y, x + w, y, r);
    g.closePath();
  },
};
