import { defineConfig, fontProviders } from "astro/config";

// Deployment is environment driven so the same build works on GitHub Pages
// (project subpath) and on Vercel or a custom domain (root path):
//
//   SITE_URL=https://alevidals.github.io SITE_BASE=/peekr   (default)
//   SITE_URL=https://peekr.app          SITE_BASE=/         (custom domain)
const site = process.env.SITE_URL || "https://alevidals.github.io";
const base = process.env.SITE_BASE || "/peekr";

export default defineConfig({
  site,
  base,
  output: "static",
  trailingSlash: "ignore",
  build: { format: "directory" },
  devToolbar: { enabled: false },
  fonts: [
    {
      provider: fontProviders.google(),
      name: "Instrument Serif",
      cssVariable: "--font-display",
      weights: ["400"],
      styles: ["normal", "italic"],
      subsets: ["latin"],
      fallbacks: ["serif"],
      optimizedFallbacks: false,
    },
    {
      provider: fontProviders.google(),
      name: "DM Sans",
      cssVariable: "--font-sans",
      weights: ["400 700"],
      styles: ["normal"],
      subsets: ["latin"],
      fallbacks: ["sans-serif"],
      optimizedFallbacks: false,
    },
    {
      provider: fontProviders.google(),
      name: "IBM Plex Mono",
      cssVariable: "--font-mono",
      weights: ["400", "500"],
      styles: ["normal"],
      subsets: ["latin"],
      fallbacks: ["monospace"],
      optimizedFallbacks: false,
    },
  ],
});
