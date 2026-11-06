import { useLayoutEffect, useRef, type ReactNode } from "react";

/** Keep the rendered menu inside its canvas, including enlarged text/themes. */
export function CanvasContextMenu({
  x,
  y,
  label,
  onClose,
  children,
}: {
  x: number;
  y: number;
  label: string;
  onClose: () => void;
  children: ReactNode;
}) {
  const menuRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const menu = menuRef.current;
    const container = menu?.parentElement;
    if (!menu || !container) return;
    const place = () => {
      menu.style.left = `${Math.max(8, Math.min(x, container.clientWidth - menu.offsetWidth - 8))}px`;
      menu.style.top = `${Math.max(8, Math.min(y, container.clientHeight - menu.offsetHeight - 8))}px`;
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(menu);
    observer.observe(container);
    return () => observer.disconnect();
  }, [x, y]);

  return (
    <div className="canvas-context-backdrop" onPointerDown={onClose}>
      <div
        ref={menuRef}
        className="canvas-context-menu"
        role="menu"
        aria-label={label}
        onPointerDown={(event) => event.stopPropagation()}
        onClick={(event) => event.stopPropagation()}
      >
        {children}
      </div>
    </div>
  );
}
