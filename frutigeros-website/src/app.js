/**
 * FrutigerOS website — interactive behaviour.
 * Vanilla JS, no framework dependencies.
 */

// ── Clock: update the mockup clock every second ────────────────────────────
function updateClock() {
  const el = document.querySelector('.panel-clock');
  if (!el) return;
  const now = new Date();
  const h = String(now.getHours()).padStart(2, '0');
  const m = String(now.getMinutes()).padStart(2, '0');
  el.textContent = `${h}:${m}`;
}

// ── Scroll-based nav opacity ───────────────────────────────────────────────
function handleNavScroll() {
  const nav = document.querySelector('.nav');
  if (!nav) return;
  if (window.scrollY > 20) {
    nav.style.borderBottomColor = 'rgba(0, 174, 239, 0.2)';
  } else {
    nav.style.borderBottomColor = 'rgba(0, 174, 239, 0.12)';
  }
}

// ── Intersection observer: fade-in sections on scroll ─────────────────────
function initScrollFade() {
  const items = document.querySelectorAll(
    '.feature-card, .roadmap-item, .aerowm-code-panel, .aerowm-text'
  );
  if (!items.length) return;

  items.forEach((el) => {
    el.style.opacity = '0';
    el.style.transform = 'translateY(20px)';
    el.style.transition = 'opacity 0.5s ease, transform 0.5s ease';
  });

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          entry.target.style.opacity = '1';
          entry.target.style.transform = 'translateY(0)';
          observer.unobserve(entry.target);
        }
      });
    },
    { threshold: 0.1, rootMargin: '0px 0px -40px 0px' }
  );

  items.forEach((el) => observer.observe(el));
}

// ── Smooth scroll for anchor links ─────────────────────────────────────────
function initSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach((link) => {
    link.addEventListener('click', (e) => {
      const id = link.getAttribute('href').slice(1);
      const target = document.getElementById(id);
      if (!target) return;
      e.preventDefault();
      target.scrollIntoView({ behavior: 'smooth', block: 'start' });
    });
  });
}

// ── Roadmap workspace dot cycling (cosmetic) ───────────────────────────────
function initWorkspaceDots() {
  const dots = document.querySelectorAll('.ws-dot');
  if (!dots.length) return;
  let active = 0;
  setInterval(() => {
    dots.forEach((d) => d.classList.remove('ws-active'));
    active = (active + 1) % dots.length;
    dots[active].classList.add('ws-active');
  }, 3000);
}

// ── Init ────────────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
  updateClock();
  setInterval(updateClock, 1000);

  handleNavScroll();
  window.addEventListener('scroll', handleNavScroll, { passive: true });

  initScrollFade();
  initSmoothScroll();
  initWorkspaceDots();
});
