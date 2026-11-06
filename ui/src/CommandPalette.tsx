import { useEffect, useMemo, useRef, useState } from "react";
import { paletteResults, type PaletteItem } from "./paletteItems";
import { Icon } from "./icons";

/** Ctrl+K: find an action, tool, setting, card or session. Nothing here can delete anything. */
export function CommandPalette({ items, onClose }: { items: readonly PaletteItem[]; onClose: () => void }) {
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const results = useMemo(() => paletteResults(items, query), [items, query]);
  const current = results[Math.min(active, Math.max(results.length - 1, 0))];

  useEffect(() => {
    const previous = document.activeElement;
    input.current?.focus();
    return () => {
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  useEffect(() => {
    list.current?.querySelector(".is-active")?.scrollIntoView?.({ block: "nearest" });
  }, [current?.id]);

  const choose = (item: PaletteItem | undefined) => {
    if (!item || item.disabled) return;
    onClose();
    item.run();
  };
  let lastGroup: string | null = null;
  return (
    <div className="dialog-backdrop command-palette-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="command-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Find anything"
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === "Escape") {
            event.preventDefault();
            onClose();
          } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            if (results.length > 0) {
              const step = event.key === "ArrowDown" ? 1 : results.length - 1;
              setActive((Math.min(active, results.length - 1) + step) % results.length);
            }
          } else if (event.key === "Enter") {
            event.preventDefault();
            choose(current);
          } else if (event.key === "Tab") {
            event.preventDefault();
          }
        }}
      >
        <label className="command-palette-search">
          <Icon name="search" size={17} />
          <input
            ref={input}
            role="combobox"
            aria-expanded="true"
            aria-controls="command-palette-results"
            aria-activedescendant={current ? `palette-${current.id}` : undefined}
            aria-label="Find a tool, setting, card or session"
            placeholder="Find a tool, setting, card or session"
            value={query}
            onChange={(event) => {
              setQuery(event.target.value.slice(0, 80));
              setActive(0);
            }}
          />
        </label>
        <div ref={list} id="command-palette-results" className="command-palette-results" role="listbox">
          {results.length === 0 && <p className="muted">Nothing matches “{query.trim()}”.</p>}
          {results.map((item) => {
            const heading = item.group !== lastGroup ? item.group : null;
            lastGroup = item.group;
            return (
              <div key={item.id} role="presentation">
                {heading && <p className="eyebrow command-palette-group">{heading}</p>}
                <div
                  id={`palette-${item.id}`}
                  role="option"
                  aria-selected={item === current}
                  aria-disabled={item.disabled || undefined}
                  className={`command-palette-item${item === current ? " is-active" : ""}`}
                  onMouseMove={() => setActive(results.indexOf(item))}
                  onClick={() => choose(item)}
                >
                  <Icon name={item.icon} size={16} />
                  <span>{item.title}</span>
                  <small>{item.detail}</small>
                </div>
              </div>
            );
          })}
        </div>
        <div className="command-palette-foot">
          <span>↑↓ move · Enter choose · Esc close</span>
          <span>Nothing is deleted from here</span>
        </div>
      </section>
    </div>
  );
}
