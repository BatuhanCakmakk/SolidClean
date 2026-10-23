import { useEffect, useState } from "react";
import type { Category } from "./api";

export const CATEGORY_ORDER: Category[] = ["media", "apps", "games", "system", "cache", "archive", "dev", "ai", "other"];

const BASE: Record<Category, string> = {
  media: "#E3925A",
  apps: "#4F86E8",
  games: "#2FAE80",
  system: "#7F8A9C",
  cache: "#CF9E1E",
  archive: "#DD6189",
  dev: "#8672E0",
  ai: "#C25AB0",
  other: "#A09A90",
};

export interface Swatch {
  /** İç içe gösterilen klasör çerçevesinin dolgusu. */
  frame: string;
  /** Kutunun dolgusu. */
  fill: string;
  stroke: string;
  text: string;
}

function mix(a: string, b: string, weight: number): string {
  const pa = parseInt(a.slice(1), 16);
  const pb = parseInt(b.slice(1), 16);
  const channel = (shift: number) =>
    Math.round(((pa >> shift) & 255) * (1 - weight) + ((pb >> shift) & 255) * weight);
  return `rgb(${channel(16)}, ${channel(8)}, ${channel(0)})`;
}

const cache = new Map<string, Swatch>();

export function swatch(category: Category, dark: boolean): Swatch {
  const key = category + (dark ? ":d" : ":l");
  let s = cache.get(key);
  if (!s) {
    const base = BASE[category];
    s = dark
      ? { frame: mix(base, "#1B1B1F", 0.86), fill: mix(base, "#1B1B1F", 0.7), stroke: mix(base, "#1B1B1F", 0.3), text: mix(base, "#FFFFFF", 0.55) }
      : { frame: mix(base, "#FFFFFF", 0.9), fill: mix(base, "#FFFFFF", 0.76), stroke: mix(base, "#FFFFFF", 0.38), text: mix(base, "#000000", 0.5) };
    cache.set(key, s);
  }
  return s;
}

export const categoryColor = (category: Category) => BASE[category];

/** Geliştirmede `?theme=light|dark` ile sabitlenebilir (Store görselleri için). */
const forcedTheme = import.meta.env.DEV ? new URLSearchParams(location.search).get("theme") : null;

export function useDarkMode(): boolean {
  const [dark, setDark] = useState(() => (forcedTheme ? forcedTheme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches));
  useEffect(() => {
    if (forcedTheme) return;
    const query = matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => setDark(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return dark;
}
