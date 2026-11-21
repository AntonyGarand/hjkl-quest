// Plausible pageviews -> stats.garand.dev (own site + "garand.dev" rollup).
// Opt out: localStorage.plausible_ignore = "true".
(function () {
  if (!/(^|\.)garand\.dev$/.test(location.hostname)) return;
  try {
    if (localStorage.getItem("plausible_ignore") === "true") return;
  } catch (e) {}
  var last;
  function pageview() {
    if (location.href === last) return;
    last = location.href;
    fetch("https://stats.garand.dev/api/event", {
      method: "POST",
      keepalive: true,
      headers: { "Content-Type": "text/plain" },
      body: JSON.stringify({ n: "pageview", u: last, d: location.hostname + ",garand.dev", r: document.referrer || null }),
    }).catch(function () {});
  }
  pageview();
  addEventListener("popstate", pageview);
})();
