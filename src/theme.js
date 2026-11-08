// Runs before the page paints, in every window, so the chosen theme never flashes the wrong colours.
(function () {
  const KEY = "zap-theme";
  const systemDark = window.matchMedia("(prefers-color-scheme: dark)");

  function read() {
    try {
      return localStorage.getItem(KEY) || "system";
    } catch {
      return "system";
    }
  }

  function apply() {
    const choice = read();
    document.documentElement.dataset.theme = choice === "system" ? (systemDark.matches ? "dark" : "light") : choice;
  }

  apply();
  systemDark.addEventListener("change", apply);
  // The other Zap windows follow a change made in Settings.
  window.addEventListener("storage", (e) => { if (e.key === KEY) apply(); });
  window.addEventListener("focus", apply);

  window.zapTheme = {
    get: read,
    set(choice) {
      try {
        localStorage.setItem(KEY, choice);
      } catch {}
      apply();
    },
  };
})();
