// FinTrack charts: draws every <canvas data-chart="{…}"> from the JSON in that
// attribute (written by views/charts on the server), using Chart.js. Colours
// come from the page's CSS tokens, so charts follow the light/dark theme.
"use strict";

const moneyFormat = new Intl.NumberFormat("en-CA", {
  style: "currency",
  currency: "CAD",
  maximumFractionDigits: 0,
});
const axisFormat = new Intl.NumberFormat("en-CA", {
  style: "currency",
  currency: "CAD",
  notation: "compact",
  maximumFractionDigits: 1,
});

// CSS tokens are light-dark() values; resolve them through a hidden element
// to get the colour actually in effect.
let probe = null;
function cssColor(token) {
  if (!probe || !probe.isConnected) {
    probe = document.createElement("span");
    probe.hidden = true;
    document.body.append(probe);
  }
  probe.style.color = `var(${token})`;
  return getComputedStyle(probe).color;
}

function datasets(spec) {
  const surface = cssColor("--surface");
  const bars = spec.bars.map((s) => ({
    type: "bar",
    label: s.name,
    data: s.values,
    links: s.links,
    backgroundColor: cssColor(s.color),
    borderColor: surface,
    borderWidth: spec.stacked ? { top: 2 } : 0,
    borderRadius: 4,
    borderSkipped: "start",
    stack: spec.stacked ? "total" : undefined,
    order: 2,
  }));
  const lines = spec.lines.map((s) => ({
    type: "line",
    label: s.name,
    data: s.values,
    links: s.links,
    borderColor: cssColor(s.color),
    backgroundColor: cssColor(s.color),
    borderWidth: 2,
    pointRadius: s.values.length > 60 ? 0 : 3,
    pointHoverRadius: 5,
    tension: 0.25,
    order: 1,
  }));
  return bars.concat(lines);
}

// The link under the pointer, if the clicked bar or point has one.
function linkAt(chart, event) {
  const [hit] = chart.getElementsAtEventForMode(event, "nearest", { intersect: true }, false);
  return hit ? chart.data.datasets[hit.datasetIndex].links?.[hit.index] : undefined;
}

function config(spec) {
  const data = { labels: spec.labels, datasets: datasets(spec) };
  const muted = cssColor("--muted");
  const axis = (extra) => ({
    stacked: spec.stacked,
    ticks: { color: muted },
    border: { display: false },
    ...extra,
  });
  return {
    data,
    options: {
      responsive: true,
      maintainAspectRatio: false,
      animation: { duration: 250 },
      interaction: { mode: "index", intersect: false },
      scales: {
        x: axis({ grid: { display: false }, ticks: { color: muted, maxRotation: 0 } }),
        y: axis({
          grid: { color: cssColor("--grid") },
          ticks: { color: muted, callback: (v) => axisFormat.format(v) },
        }),
      },
      plugins: {
        legend: {
          display: data.datasets.length > 1,
          position: "bottom",
          labels: {
            color: cssColor("--text-2"),
            boxWidth: 12,
            boxHeight: 12,
            // Series order, not drawing order (lines are drawn on top of bars).
            sort: (a, b) => a.datasetIndex - b.datasetIndex,
          },
        },
        tooltip: {
          callbacks: {
            label: (ctx) => ` ${ctx.dataset.label}: ${moneyFormat.format(ctx.parsed.y)}`,
            footer: (items) => {
              if (!spec.stacked || items.length < 2) return "";
              const total = items.reduce((sum, i) => sum + i.parsed.y, 0);
              return `Total: ${moneyFormat.format(total)}`;
            },
          },
        },
      },
      onHover: (event, _elements, chart) => {
        chart.canvas.style.cursor = linkAt(chart, event) ? "pointer" : "default";
      },
      onClick: (event, _elements, chart) => {
        const href = linkAt(chart, event);
        if (href) window.location.assign(href);
      },
    },
  };
}

function drawCharts(root) {
  // Charts whose canvas was swapped out by htmx are no longer needed.
  for (const chart of Object.values(Chart.instances)) {
    if (!chart.canvas.isConnected) chart.destroy();
  }
  Chart.defaults.font.family = getComputedStyle(document.body).fontFamily;
  for (const canvas of root.querySelectorAll("canvas[data-chart]")) {
    Chart.getChart(canvas)?.destroy();
    new Chart(canvas, config(JSON.parse(canvas.dataset.chart)));
  }
}

if (window.Chart) {
  if (window.htmx) {
    htmx.onLoad(drawCharts); // the first page, and every page htmx swaps in
  } else {
    document.addEventListener("DOMContentLoaded", () => drawCharts(document.body));
  }
  // Redraw with the other palette when the device switches light/dark.
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    drawCharts(document.body);
  });
}
