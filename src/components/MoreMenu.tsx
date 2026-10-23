import { useEffect, useRef, useState } from "react";
import { t } from "../i18n";

interface Props {
  onAbout: () => void;
  onSupport: () => void;
}

/** Üst çubuğun sağındaki "⋯" menüsü: uygulamanın kendisiyle ilgili, sık kullanılmayan komutlar. */
export function MoreMenu({ onAbout, onSupport }: Props) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const toggle = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const items = () => [...(root.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
    items()[0]?.focus();
    const onPointer = (e: MouseEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" || e.key === "Tab") {
        setOpen(false);
        if (e.key === "Escape") toggle.current?.focus();
      } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const list = items();
        const i = list.indexOf(document.activeElement as HTMLButtonElement);
        list[(i + (e.key === "ArrowDown" ? 1 : list.length - 1)) % list.length]?.focus();
      }
    };
    document.addEventListener("mousedown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const choose = (action: () => void) => () => {
    setOpen(false);
    action();
  };

  return (
    <div className="more" ref={root}>
      <button
        ref={toggle}
        className="more-toggle"
        aria-label={t.moreOptions}
        title={t.moreOptions}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span aria-hidden="true">&#xE712;</span>
      </button>
      {open && (
        <div className="more-menu" role="menu" aria-label={t.moreOptions}>
          <button role="menuitem" onClick={choose(onAbout)}>
            <span className="more-icon glyph" aria-hidden="true">
              &#xE946;
            </span>
            {t.about}
          </button>
          <div className="more-sep" role="separator" />
          <button role="menuitem" title={t.supportHint} onClick={choose(onSupport)}>
            <span className="more-icon" aria-hidden="true">
              ☕
            </span>
            {t.support}
          </button>
        </div>
      )}
    </div>
  );
}
