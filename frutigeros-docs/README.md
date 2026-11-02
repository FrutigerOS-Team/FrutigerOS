# FrutigerOS Documentation

Static documentation portal. No build step, no runtime dependencies.

## Preview

```bash
cd frutigeros-docs
python3 -m http.server 8080
# http://localhost:8080/
```

The site also works directly from `file://`, because every link is relative.

## Pages

| File | Topic |
|------|-------|
| `index.html` | Overview, project status, design principles |
| `pages/getting-started.html` | Build and test the Rust components |
| `pages/installation.html` | ISO build and installer usage |
| `pages/aerowm.html` | Compositor internals, keybindings, configuration |
| `pages/aeroshell.html` | Panel client, renderer, launcher, notifications |
| `pages/applications.html` | Native app inventory and backends |
| `pages/architecture.html` | Component boundaries and packaging plan |
| `pages/troubleshooting.html` | Symptoms and fixes |
| `pages/contributing.html` | Contribution workflow |

## Assets

`assets/logo.svg`, `assets/banner.svg` and `assets/architecture.svg` are original
SVG artwork created for this project. `assets/style.css` implements the Frutiger
Aero design language; `assets/docs.js` adds theme switching, navigation filtering,
copy buttons and scroll reveals.

## Editing

Keep the top bar and sidebar markup consistent between pages. When adding a page:

1. Copy the structure of an existing page.
2. Add it to the sidebar in every page (the navigation is static, not injected).
3. Add a previous/next link on its neighbours.
4. Verify all relative links resolve.
