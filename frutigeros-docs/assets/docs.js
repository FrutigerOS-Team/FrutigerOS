/* FrutigerOS docs portal — progressive enhancement only. */
(() => {
  const root = document.documentElement;
  const body = document.body;

  // ── Theme ────────────────────────────────────────────────────────────────
  const stored = localStorage.getItem("frutigeros-theme");
  const prefersLight = window.matchMedia("(prefers-color-scheme: light)").matches;
  const initial = stored || (prefersLight ? "light" : "dark");
  root.dataset.theme = initial;

  const themeButton = document.querySelector("[data-theme-toggle]");
  const syncThemeLabel = () => {
    if (!themeButton) return;
    const dark = root.dataset.theme !== "light";
    themeButton.textContent = dark ? "☀" : "☾";
    themeButton.setAttribute("aria-label", dark ? "Switch to light theme" : "Switch to dark theme");
    themeButton.setAttribute("title", dark ? "Light theme" : "Dark theme");
  };
  syncThemeLabel();

  themeButton?.addEventListener("click", () => {
    root.dataset.theme = root.dataset.theme === "light" ? "dark" : "light";
    localStorage.setItem("frutigeros-theme", root.dataset.theme);
    syncThemeLabel();
  });

  // ── Mobile navigation ────────────────────────────────────────────────────
  const menuButton = document.querySelector("[data-menu-toggle]");
  menuButton?.addEventListener("click", () => {
    body.classList.toggle("nav-open");
    menuButton.setAttribute("aria-expanded", String(body.classList.contains("nav-open")));
  });
  document.querySelectorAll(".sidebar a").forEach((link) => {
    link.addEventListener("click", () => body.classList.remove("nav-open"));
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") body.classList.remove("nav-open");
    if (event.key === "/" && document.activeElement?.tagName !== "INPUT") {
      event.preventDefault();
      document.querySelector(".search input")?.focus();
    }
  });

  // ── Navigation filter ────────────────────────────────────────────────────
  const search = document.querySelector(".search input");
  const links = Array.from(document.querySelectorAll(".nav-group a"));
  const groups = Array.from(document.querySelectorAll(".nav-group"));
  let empty = null;

  const filterNav = () => {
    const query = (search?.value || "").trim().toLowerCase();
    let visible = 0;
    links.forEach((link) => {
      const match = !query || link.textContent.toLowerCase().includes(query);
      link.parentElement.hidden = !match;
      if (match) visible += 1;
    });
    groups.forEach((group) => {
      const any = Array.from(group.querySelectorAll("li")).some((li) => !li.hidden);
      group.hidden = !any;
    });
    if (query && visible === 0) {
      if (!empty) {
        empty = document.createElement("p");
        empty.className = "nav-empty";
        empty.textContent = "No matching pages.";
        document.querySelector(".sidebar")?.appendChild(empty);
      }
      empty.hidden = false;
    } else if (empty) {
      empty.hidden = true;
    }
  };
  search?.addEventListener("input", filterNav);

  // ── Code copy buttons ────────────────────────────────────────────────────
  document.querySelectorAll("pre").forEach((pre) => {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "copy-button";
    button.textContent = "Copy";
    button.addEventListener("click", async () => {
      const text = pre.querySelector("code")?.innerText ?? pre.innerText;
      try {
        await navigator.clipboard.writeText(text);
        button.textContent = "Copied";
        setTimeout(() => (button.textContent = "Copy"), 1400);
      } catch {
        button.textContent = "Select";
      }
    });
    pre.appendChild(button);
  });

  // ── Reveal on scroll ─────────────────────────────────────────────────────
  const revealables = document.querySelectorAll(".card, .note, .hero, .content h2");
  if ("IntersectionObserver" in window) {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            entry.target.classList.add("visible");
            observer.unobserve(entry.target);
          }
        });
      },
      { rootMargin: "0px 0px -8% 0px" }
    );
    revealables.forEach((el) => {
      el.classList.add("reveal");
      observer.observe(el);
    });
  }
})();
