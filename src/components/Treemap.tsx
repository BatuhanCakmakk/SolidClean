import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { TreeNode } from "../api";
import { swatch, useDarkMode } from "../categories";
import { formatBytes, formatCount } from "../format";
import { t } from "../i18n";
import { squarify, type Rect } from "../squarify";

export interface Tile {
  node: TreeNode;
  rect: Rect;
  /** Gösterilen klasöre göre çocuk sıra numaraları; gezinilemeyen öğelerde boş. */
  path: number[];
  names: string[];
  level: 0 | 1;
  /** İçinde alt kutular çizilen çerçeve mi? */
  nested: boolean;
}

const GAP = 3;
const HEADER = 24;
const PAD = 5;
// Tuval CSS değişkenini okuyamaz; dile göre seçilen yazı tipi (styles.css --font) çizim anında alınır.
let font = "";
const uiFont = () => (font ||= getComputedStyle(document.documentElement).getPropertyValue("--font").trim());

export function tileLabel(node: TreeNode): string {
  if (node.kind === "small") return t.smallFiles(formatCount(node.files));
  if (node.kind === "rest") return t.more(formatCount(node.childCount));
  return node.name;
}

const inset = (r: Rect, d: number): Rect => ({ x: r.x + d, y: r.y + d, w: r.w - 2 * d, h: r.h - 2 * d });
const visible = (nodes: TreeNode[]) => nodes.filter((n) => n.alloc > 0);

function layoutTiles(root: TreeNode, width: number, height: number): Tile[] {
  const tiles: Tile[] = [];
  for (const { item, rect } of squarify(visible(root.children), (n) => n.alloc, { x: 0, y: 0, w: width, h: height })) {
    const r = inset(rect, GAP / 2);
    if (r.w < 1 || r.h < 1) continue;
    const kids = visible(item.children);
    const nested = kids.length > 0 && r.w >= 90 && r.h >= HEADER + 40;
    tiles.push({ node: item, rect: r, path: item.index >= 0 ? [item.index] : [], names: [item.name], level: 0, nested });
    if (!nested) continue;
    const inner = { x: r.x + PAD, y: r.y + HEADER, w: r.w - 2 * PAD, h: r.h - HEADER - PAD };
    for (const { item: kid, rect: kr } of squarify(kids, (n) => n.alloc, inner)) {
      const ir = inset(kr, 1.5);
      if (ir.w < 1 || ir.h < 1) continue;
      const path = item.index >= 0 && kid.index >= 0 ? [item.index, kid.index] : [];
      tiles.push({ node: kid, rect: ir, path, names: [item.name, kid.name], level: 1, nested: false });
    }
  }
  return tiles;
}

function roundRect(ctx: CanvasRenderingContext2D, r: Rect, radius: number) {
  const rad = Math.max(0, Math.min(radius, r.w / 2, r.h / 2));
  ctx.beginPath();
  ctx.roundRect(r.x, r.y, r.w, r.h, rad);
}

function fitText(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string {
  if (maxWidth <= 0) return "";
  if (ctx.measureText(text).width <= maxWidth) return text;
  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (ctx.measureText(text.slice(0, mid) + "…").width <= maxWidth) lo = mid;
    else hi = mid - 1;
  }
  return lo > 0 ? text.slice(0, lo) + "…" : "";
}

export const HATCH = {
  safe: { line: "rgba(47, 174, 128, 0.55)", border: "#2FAE80" },
  review: { line: "rgba(214, 160, 20, 0.6)", border: "#D6A014" },
};

/**
 * Temizlik önerisi kapsamındaki kutuların çapraz taraması. Çizgiler tuvalin
 * koordinatlarına hizalı, böylece iç içe kutularda üst üste binince tek desen gibi görünür.
 */
function hatch(ctx: CanvasRenderingContext2D, r: Rect, radius: number, color: string) {
  const step = 7;
  ctx.save();
  roundRect(ctx, r, radius);
  ctx.clip();
  ctx.beginPath();
  for (let c = Math.floor((r.x + r.y) / step) * step; c < r.x + r.w + r.y + r.h; c += step) {
    ctx.moveTo(c - r.y, r.y);
    ctx.lineTo(c - r.y - r.h, r.y + r.h);
  }
  ctx.lineWidth = 1.2;
  ctx.strokeStyle = color;
  ctx.stroke();
  ctx.restore();
}

function draw(ctx: CanvasRenderingContext2D, tiles: Tile[], width: number, height: number, dark: boolean, hovered: Tile | null) {
  const strong = dark ? "#EEECE8" : "#1D1D1F";
  const muted = dark ? "#A19D96" : "#6E6A64";
  ctx.clearRect(0, 0, width, height);
  ctx.textBaseline = "alphabetic";

  for (const tile of tiles) {
    const { rect: r, node } = tile;
    const neutral = node.kind === "small" || node.kind === "rest";
    const sw = swatch(neutral ? "other" : node.category, dark);
    const radius = tile.level === 0 ? 7 : 4;
    const marked = node.cleanup ? HATCH[node.cleanup] : null;
    roundRect(ctx, r, radius);
    ctx.fillStyle = tile.nested ? sw.frame : sw.fill;
    ctx.fill();
    if (marked) hatch(ctx, r, radius, marked.line);
    roundRect(ctx, r, radius);
    ctx.lineWidth = marked ? 1.5 : 1;
    ctx.strokeStyle = marked ? marked.border : sw.stroke;
    ctx.stroke();

    ctx.save();
    ctx.beginPath();
    ctx.rect(r.x, r.y, r.w, r.h);
    ctx.clip();
    const label = tileLabel(node);
    const size = formatBytes(node.alloc);
    if (tile.nested) {
      ctx.font = `600 12px ${uiFont()}`;
      const sizeWidth = ctx.measureText(size).width;
      ctx.fillStyle = muted;
      ctx.fillText(size, r.x + r.w - 8 - sizeWidth, r.y + 16);
      ctx.fillStyle = strong;
      ctx.fillText(fitText(ctx, label, r.w - sizeWidth - 22), r.x + 8, r.y + 16);
    } else if (r.w > 34 && r.h > 18) {
      ctx.font = `${tile.level === 0 ? 600 : 400} ${tile.level === 0 ? 12 : 11}px ${uiFont()}`;
      ctx.fillStyle = tile.level === 0 ? strong : sw.text;
      ctx.fillText(fitText(ctx, label, r.w - 12), r.x + 6, r.y + 15);
      if (r.h > 44 && r.w > 52) {
        const fontSize = Math.round(Math.max(11, Math.min(22, r.w / 7, r.h / 3.2)));
        ctx.font = `600 ${fontSize}px ${uiFont()}`;
        ctx.fillStyle = strong;
        ctx.fillText(fitText(ctx, size, r.w - 12), r.x + 6, r.y + r.h - 8);
      }
    }
    ctx.restore();
  }

  if (hovered) {
    roundRect(ctx, hovered.rect, hovered.level === 0 ? 7 : 4);
    ctx.lineWidth = 2;
    ctx.strokeStyle = dark ? "#FFFFFF" : "#1D1D1F";
    ctx.stroke();
  }
}

interface Props {
  root: TreeNode;
  onOpen: (tile: Tile) => void;
  onHover: (tile: Tile | null) => void;
}

export function Treemap({ root, onOpen, onHover }: Props) {
  const wrapRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const hoveredRef = useRef<Tile | null>(null);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const dark = useDarkMode();

  const tiles = useMemo(() => (size.w > 0 && size.h > 0 ? layoutTiles(root, size.w, size.h) : []), [root, size]);

  const redraw = useCallback(() => {
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    const pw = Math.round(size.w * dpr);
    const ph = Math.round(size.h * dpr);
    if (canvas.width !== pw || canvas.height !== ph) {
      canvas.width = pw;
      canvas.height = ph;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw(ctx, tiles, size.w, size.h, dark, hoveredRef.current);
  }, [tiles, size, dark]);

  useEffect(() => {
    const wrap = wrapRef.current;
    if (!wrap) return;
    const observer = new ResizeObserver(([entry]) => {
      setSize({ w: Math.floor(entry.contentRect.width), h: Math.floor(entry.contentRect.height) });
    });
    observer.observe(wrap);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    hoveredRef.current = null;
    redraw();
  }, [redraw]);

  // Sonra eklenen (daha derindeki) kutu önceliklidir.
  const hit = (x: number, y: number): Tile | null => {
    for (let i = tiles.length - 1; i >= 0; i--) {
      const r = tiles[i].rect;
      if (x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h) return tiles[i];
    }
    return null;
  };

  const pointer = (e: React.MouseEvent) => {
    const bounds = e.currentTarget.getBoundingClientRect();
    return hit(e.clientX - bounds.left, e.clientY - bounds.top);
  };

  const setHovered = (tile: Tile | null) => {
    if (tile === hoveredRef.current) return;
    hoveredRef.current = tile;
    onHover(tile);
    redraw();
  };

  return (
    <div className="treemap" ref={wrapRef}>
      <canvas
        ref={canvasRef}
        style={{ width: size.w, height: size.h, cursor: hoveredRef.current?.node.kind === "dir" ? "pointer" : "default" }}
        onMouseMove={(e) => setHovered(pointer(e))}
        onMouseLeave={() => setHovered(null)}
        onClick={(e) => {
          const tile = pointer(e);
          if (tile) onOpen(tile);
        }}
      />
    </div>
  );
}
