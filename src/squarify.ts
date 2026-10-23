export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Placed<T> {
  item: T;
  rect: Rect;
}

/**
 * Squarified treemap yerleşimi (Bruls, Huizing, van Wijk). Kutuları kareye yakın
 * tutar, böylece etiketler okunur kalır. `items` büyükten küçüğe sıralı ve
 * değerleri sıfırdan büyük olmalı.
 */
export function squarify<T>(items: T[], value: (item: T) => number, bounds: Rect): Placed<T>[] {
  const total = items.reduce((sum, item) => sum + value(item), 0);
  if (total <= 0 || bounds.w <= 0 || bounds.h <= 0) return [];
  const scale = (bounds.w * bounds.h) / total;
  const areas = items.map((item) => value(item) * scale);
  const out: Placed<T>[] = [];
  let rect = { ...bounds };
  let i = 0;

  while (i < items.length) {
    const side = Math.min(rect.w, rect.h);
    let end = i + 1;
    let sum = areas[i];
    let min = areas[i];
    let max = areas[i];
    let best = worstRatio(sum, min, max, side);
    while (end < items.length) {
      const a = areas[end];
      const ratio = worstRatio(sum + a, Math.min(min, a), Math.max(max, a), side);
      if (ratio > best) break;
      sum += a;
      min = Math.min(min, a);
      max = Math.max(max, a);
      best = ratio;
      end++;
    }

    // Son satır kalan alanı tam doldursun, yuvarlama boşluğu kalmasın.
    const last = end === items.length;
    if (rect.w >= rect.h) {
      const width = last ? rect.w : sum / rect.h;
      let y = rect.y;
      for (let k = i; k < end; k++) {
        const h = (areas[k] / sum) * rect.h;
        out.push({ item: items[k], rect: { x: rect.x, y, w: width, h } });
        y += h;
      }
      rect = { x: rect.x + width, y: rect.y, w: rect.w - width, h: rect.h };
    } else {
      const height = last ? rect.h : sum / rect.w;
      let x = rect.x;
      for (let k = i; k < end; k++) {
        const w = (areas[k] / sum) * rect.w;
        out.push({ item: items[k], rect: { x, y: rect.y, w, h: height } });
        x += w;
      }
      rect = { x: rect.x, y: rect.y + height, w: rect.w, h: rect.h - height };
    }
    i = end;
  }
  return out;
}

function worstRatio(sum: number, min: number, max: number, side: number): number {
  const s2 = sum * sum;
  const w2 = side * side;
  return Math.max((w2 * max) / s2, s2 / (w2 * min));
}
