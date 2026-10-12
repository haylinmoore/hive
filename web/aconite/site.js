(() => {
  "use strict";
  const PERI = "#5566bb";
  const PLUM = "#7a4a8f";
  const CA = "#c8c8c8";
  const CB = "#bababa";
  const MAIL = "hello@aconite.systems";
  const $ = (s) => document.querySelector(s);
  const $$ = (s) => [...document.querySelectorAll(s)];
  const fmt = (n) => n.toLocaleString("en-US");
  const money = (n) => "$" + fmt(n);
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;

  function d2xy(n, d) {
    let x = 0;
    let y = 0;
    let t = d;
    for (let s = 1; s < n; s *= 2) {
      const rx = 1 & (t / 2);
      const ry = 1 & (t ^ rx);
      if (ry === 0) {
        if (rx === 1) {
          x = s - 1 - x;
          y = s - 1 - y;
        }
        [x, y] = [y, x];
      }
      x += s * rx;
      y += s * ry;
      t = Math.floor(t / 4);
    }
    return [x, y];
  }

  const BLOCKS = {
    "v4-22": {
      name: "ipv4 /22",
      cells: 1024,
      count: 1024,
      unit: "addresses",
      cap: "Each square is one address.",
      sn: 32,
      mo: 350,
      yr: 3500,
      prefix: "a.b.c.0/22",
      ml: "/24",
      zone: "c.b.a.in-addr.arpa (x4)",
      six: false,
    },
    "v4-24": {
      name: "ipv4 /24",
      cells: 256,
      count: 256,
      unit: "addresses",
      cap: "Each square is one address.",
      sn: 16,
      mo: 100,
      yr: 1000,
      prefix: "a.b.c.0/24",
      ml: "/24",
      zone: "c.b.a.in-addr.arpa",
      six: false,
    },
    "v6-44": {
      name: "ipv6 /44",
      cells: 1024,
      count: 1048576,
      unit: "/64 networks",
      cap: "Each square is 1,024 /64 networks.",
      sn: 32,
      mo: 5,
      yr: 30,
      prefix: "2001:db8::/44",
      ml: "/48",
      zone: "0.0.0.8.b.d.0.1.0.0.2.ip6.arpa",
      six: true,
    },
  };

  const fc = $("#field");
  const fx = fc.getContext("2d");
  const GRIDS = {};
  for (const sn of [16, 32]) {
    const sheet = Array.from({ length: sn * sn }, (_, i) => d2xy(sn, i));
    GRIDS[sn] = {
      sn,
      fs: fc.width / sn,
      sheet,
      inv: new Map(sheet.map(([x, y], i) => [y * sn + x, i])),
    };
  }
  const cap = $("#fieldCap");
  let cur = "v4-24";
  let term = "mo";
  let shown = 0;
  let target = 0;
  let t0 = 0;
  const MS_PER_24 = 1600;
  let hoverI = -1;

  function drawField() {
    const b = BLOCKS[cur];
    const { sn: SN, fs: FS, sheet: SHEET } = GRIDS[b.sn];
    for (let y = 0; y < SN; y++)
      for (let x = 0; x < SN; x++) {
        fx.fillStyle = (x + y) & 1 ? CB : CA;
        fx.fillRect(x * FS, y * FS, FS, FS);
      }
    fx.fillStyle = b.six ? PLUM : PERI;
    for (let i = 0; i < shown; i++) {
      const [x, y] = SHEET[i];
      fx.fillRect(x * FS, y * FS, FS, FS);
    }
    fx.strokeStyle = "rgba(255,255,255,.16)";
    fx.lineWidth = 1;
    for (let i = 0; i < shown; i++) {
      const [x, y] = SHEET[i];
      fx.strokeRect(x * FS + 0.5, y * FS + 0.5, FS - 1, FS - 1);
    }
    if (SN === 32) {
      fx.strokeStyle = "#fff";
      fx.lineWidth = 3;
      fx.beginPath();
      fx.moveTo(fc.width / 2, 0);
      fx.lineTo(fc.width / 2, fc.height);
      fx.moveTo(0, fc.height / 2);
      fx.lineTo(fc.width, fc.height / 2);
      fx.stroke();
    }
    if (hoverI >= 0) {
      const [x, y] = SHEET[hoverI];
      fx.strokeStyle = "#1a1a1a";
      fx.strokeRect(x * FS + 1.5, y * FS + 1.5, FS - 3, FS - 3);
    }
  }

  function tick(now) {
    t0 ||= now;
    const p = Math.min(1, (now - t0) / ((MS_PER_24 * target) / 256));
    shown = Math.round(target * (1 - (1 - p) ** 3));
    drawField();
    if (p < 1) requestAnimationFrame(tick);
  }

  function fill() {
    target = BLOCKS[cur].cells;
    shown = 0;
    t0 = 0;
    if (still) {
      shown = target;
      drawField();
      return;
    }
    requestAnimationFrame(tick);
  }

  function update() {
    const b = BLOCKS[cur];
    const save = b.mo * 12 - b.yr;
    $("#slipCount").textContent = fmt(b.count);
    $("#slipUnit").textContent = b.unit;
    $("#slipPrice").textContent = money(term === "mo" ? b.mo : b.yr);
    $("#slipPer").textContent = term === "mo" ? "a month" : "a year";
    $("#slipAlt").textContent =
      term === "mo"
        ? `or ${money(b.yr)} a year, saving ${money(save)}`
        : `${money(save)} less than monthly`;
    $("#price").classList.toggle("yr", term === "yr");
    cap.textContent = b.cap;
    $("#wBlock").textContent = b.prefix;
    $$("[data-p]").forEach((e) => (e.textContent = b.prefix));
    $$("[data-l]").forEach((e) => (e.textContent = b.ml));
    $$("[data-z]").forEach((e) => (e.textContent = b.zone));
    const termName = term === "mo" ? "monthly" : "yearly";
    const body = `Hello,\n\nI'd like to lease an ${b.name}, billed ${termName}.\n\nAnnouncing ASN:\nName / company:\n`;
    $("#slipGo").href =
      `mailto:${MAIL}?subject=${encodeURIComponent("Lease request: " + b.name)}` +
      `&body=${encodeURIComponent(body)}`;
  }

  function pick(id) {
    cur = id;
    hoverI = -1;
    $$("#price tbody tr").forEach((r) =>
      r.classList.toggle("is-on", r.dataset.block === id),
    );
    update();
    fill();
  }

  $$("#price tbody tr").forEach((r) => {
    r.addEventListener("click", () => pick(r.dataset.block));
    r.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        pick(r.dataset.block);
      }
    });
  });

  $$(".term button").forEach((bt) =>
    bt.addEventListener("click", () => {
      term = bt.dataset.term;
      $$(".term button").forEach((x) => x.classList.toggle("is-on", x === bt));
      update();
    }),
  );

  function hoverAt(e) {
    const b = BLOCKS[cur];
    const { sn: SN, inv: INV } = GRIDS[b.sn];
    const r = fc.getBoundingClientRect();
    const x = Math.floor(((e.clientX - r.left) / r.width) * SN);
    const y = Math.floor(((e.clientY - r.top) / r.height) * SN);
    if (x < 0 || y < 0 || x >= SN || y >= SN) return;
    const i = INV.get(y * SN + x);
    hoverI = i;
    if (b.six) {
      cap.textContent = `/64s ${fmt(i * 1024 + 1)} to ${fmt(i * 1024 + 1024)} of the /44.`;
    } else {
      const third = Math.floor(i / 256);
      const addr = `a.b.${third ? "c+" + third : "c"}.${i % 256}`;
      cap.textContent =
        i < b.cells ? `${addr} is in the block.` : `${addr} is outside a /24.`;
    }
    drawField();
  }

  fc.addEventListener("pointermove", hoverAt);
  fc.addEventListener("pointerdown", hoverAt);
  fc.addEventListener("pointerleave", () => {
    hoverI = -1;
    cap.textContent = BLOCKS[cur].cap;
    drawField();
  });

  update();
  drawField();
  new IntersectionObserver(
    (es, io) => {
      if (es[0].isIntersecting) {
        io.disconnect();
        fill();
      }
    },
    { threshold: 0.3 },
  ).observe(fc);
})();
