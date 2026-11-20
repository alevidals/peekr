import React from "react";
import ReactDOM from "react-dom/client";
import "../App.css";
import "./preview.css";
import { THEME_OPTIONS } from "../themes";
import { DEFAULT_APPEARANCE } from "../types";
import { applyAppearance } from "../theme";
import { AppPopover } from "./AppPopover";

const params = new URLSearchParams(window.location.search);
const themeValue = params.get("theme");

function Gallery() {
  return (
    <main className="gallery">
      <header className="gallery-head">
        <p className="gallery-kicker">Temporary preview · invented data · not wired to Tauri</p>
        <h1>
          Every theme,
          <br />
          one screen.
        </h1>
        <p className="gallery-lead">
          The real popover component and CSS, once per appearance option. Tabs, search and scroll
          work inside each frame; System follows the operating system.
        </p>
      </header>

      <div className="gallery-grid">
        {THEME_OPTIONS.map((option) => (
          <section className="gallery-item" key={option.value}>
            <div className="gallery-label">
              <h2>{option.label}</h2>
              <span>{option.description}</span>
            </div>
            <iframe
              className="gallery-frame"
              title={`peekr popover · ${option.label}`}
              src={`?theme=${option.value}`}
              width={380}
              height={500}
              loading="lazy"
            />
          </section>
        ))}
      </div>

      <footer className="gallery-foot">
        <span>peekr / theme gallery</span>
        <span>
          Run <code>bun run dev:gallery</code>, or open <code>/preview.html</code> on the running
          dev server.
        </span>
      </footer>
    </main>
  );
}

const root = ReactDOM.createRoot(document.getElementById("preview-root") as HTMLElement);

if (themeValue) {
  const option = THEME_OPTIONS.find((item) => item.value === themeValue) ?? THEME_OPTIONS[0];
  // Each frame is its own document, so the app's appearance writer can set the theme
  // globally without leaking into the other frames.
  applyAppearance(option.theme, { ...DEFAULT_APPEARANCE, palette: option.palette });
  document.documentElement.dataset.view = "embed";
  root.render(
    <React.StrictMode>
      <AppPopover />
    </React.StrictMode>
  );
} else {
  document.documentElement.dataset.view = "gallery";
  root.render(
    <React.StrictMode>
      <Gallery />
    </React.StrictMode>
  );
}
