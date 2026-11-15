import { useEffect, useId, useRef, useState } from "react";
import { THEME_COLORS, type CustomTheme, type ThemeColor } from "./customTheme";
import { SliderInput } from "./Slider";
import { hexToHsv, hexToRgb, hsvToHex, hsvToRgb, type Hsv } from "./themeColor";
import {
  contrastForRequirement,
  contrastFromLuminance,
  contrastRequirementsFor,
  meetsRequirement,
  rgbLuminance,
  type ContrastRequirement,
} from "./themeContrast";

const SQUARE_SIZE = 240;

function clamp01(value: number): number {
  return Math.max(0, Math.min(1, value));
}

function hexInput(text: string): string | null {
  const match = /^#?([0-9a-f]{6})$/i.exec(text.trim());
  return match ? `#${match[1]!.toLowerCase()}` : null;
}

/** The rule that is furthest below its minimum, for the current colour. */
function worstRequirement(value: string, requirements: ContrastRequirement[]): ContrastRequirement | null {
  let worst: ContrastRequirement | null = null;
  let score = Number.POSITIVE_INFINITY;
  for (const requirement of requirements) {
    const ratio = contrastForRequirement(value, requirement);
    if (ratio === null) continue;
    const next = ratio / requirement.minimum;
    if (next < score) {
      score = next;
      worst = requirement;
    }
  }
  return worst;
}

function directionHint(current: Hsv, requirements: ContrastRequirement[]): string {
  const passesAt = (saturation: number, value: number) => {
    const candidate = hsvToHex(current.h, clamp01(saturation), clamp01(value));
    return requirements.every((requirement) => meetsRequirement(candidate, requirement));
  };
  const lighter = passesAt(current.s, current.v + 0.08);
  const darker = passesAt(current.s, current.v - 0.08);
  if (lighter && darker) return "Either a lighter or a darker shade will pass.";
  if (lighter) return "Go lighter to pass the line.";
  if (darker) return "Go darker to pass the line.";
  return "Change the hue or saturation to find the passing side.";
}

/**
 * Draw the saturation/brightness square for one hue, shade the colours that
 * miss the contrast guidance, and draw the boundary as a white line with a
 * dark halo so it stays visible on light and dark colours.
 */
function drawSquare(canvas: HTMLCanvasElement, hue: number, requirements: ContrastRequirement[]) {
  const context = canvas.getContext("2d");
  if (!context) return;
  if (canvas.width !== SQUARE_SIZE || canvas.height !== SQUARE_SIZE) {
    canvas.width = SQUARE_SIZE;
    canvas.height = SQUARE_SIZE;
  }
  const image = context.createImageData(SQUARE_SIZE, SQUARE_SIZE);
  const data = image.data;
  const pass = new Uint8Array(SQUARE_SIZE * SQUARE_SIZE);
  const fixed = requirements.flatMap((requirement) => {
    const rgb = hexToRgb(requirement.other);
    return rgb ? [{ luminance: rgbLuminance(...rgb), minimum: requirement.minimum }] : [];
  });
  for (let y = 0; y < SQUARE_SIZE; y += 1) {
    const value = 1 - y / (SQUARE_SIZE - 1);
    for (let x = 0; x < SQUARE_SIZE; x += 1) {
      const saturation = x / (SQUARE_SIZE - 1);
      const [red, green, blue] = hsvToRgb(hue, saturation, value);
      const index = y * SQUARE_SIZE + x;
      const luminance = rgbLuminance(red, green, blue);
      const meetsAll = fixed.every((entry) => contrastFromLuminance(luminance, entry.luminance) >= entry.minimum);
      pass[index] = meetsAll ? 1 : 0;
      const shade = meetsAll ? 1 : 0.55;
      data[index * 4] = Math.round(red * shade);
      data[index * 4 + 1] = Math.round(green * shade);
      data[index * 4 + 2] = Math.round(blue * shade);
      data[index * 4 + 3] = 255;
    }
  }
  if (fixed.length) {
    const edge = new Uint8Array(SQUARE_SIZE * SQUARE_SIZE);
    for (let y = 0; y < SQUARE_SIZE; y += 1) {
      for (let x = 0; x < SQUARE_SIZE; x += 1) {
        const index = y * SQUARE_SIZE + x;
        if (x + 1 < SQUARE_SIZE && pass[index] !== pass[index + 1]) edge[index] = edge[index + 1] = 1;
        if (y + 1 < SQUARE_SIZE && pass[index] !== pass[index + SQUARE_SIZE]) {
          edge[index] = edge[index + SQUARE_SIZE] = 1;
        }
      }
    }
    const halo = new Uint8Array(SQUARE_SIZE * SQUARE_SIZE);
    const core = new Uint8Array(SQUARE_SIZE * SQUARE_SIZE);
    for (let y = 0; y < SQUARE_SIZE; y += 1) {
      for (let x = 0; x < SQUARE_SIZE; x += 1) {
        if (!edge[y * SQUARE_SIZE + x]) continue;
        for (let offsetY = -2; offsetY <= 2; offsetY += 1) {
          const lineY = y + offsetY;
          if (lineY < 0 || lineY >= SQUARE_SIZE) continue;
          for (let offsetX = -2; offsetX <= 2; offsetX += 1) {
            const lineX = x + offsetX;
            if (lineX < 0 || lineX >= SQUARE_SIZE) continue;
            halo[lineY * SQUARE_SIZE + lineX] = 1;
            if (Math.abs(offsetX) <= 1 && Math.abs(offsetY) <= 1) core[lineY * SQUARE_SIZE + lineX] = 1;
          }
        }
      }
    }
    for (let index = 0; index < SQUARE_SIZE * SQUARE_SIZE; index += 1) {
      if (!halo[index]) continue;
      const channel = core[index] ? 255 : 0;
      data[index * 4] = channel;
      data[index * 4 + 1] = channel;
      data[index * 4 + 2] = channel;
    }
  }
  context.putImageData(image, 0, 0);
}

export function ThemeColorPicker({
  colorKey,
  theme,
  value,
  open,
  onOpenChange,
  onChange,
  highlighted,
}: {
  colorKey: ThemeColor;
  theme: CustomTheme;
  value: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onChange: (hex: string) => void;
  highlighted: boolean;
}) {
  const label = THEME_COLORS[colorKey];
  const inputId = `theme-color-${colorKey}`;
  const popoverId = `theme-color-picker-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  const guidanceId = `${popoverId}-guidance`;
  const [hue, setHue] = useState(() => hexToHsv(value)?.h ?? 0);
  const [draft, setDraft] = useState(value);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const squareRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const itemRef = useRef<HTMLDivElement>(null);
  const swatchRef = useRef<HTMLButtonElement>(null);
  const closeRef = useRef(onOpenChange);
  const requirements = contrastRequirementsFor(theme, colorKey);
  const requirementsRef = useRef(requirements);
  const referenceColors = requirements.map((requirement) => requirement.other).join("|");

  useEffect(() => {
    closeRef.current = onOpenChange;
  });

  useEffect(() => {
    requirementsRef.current = requirements;
  });

  useEffect(() => {
    setDraft(value);
  }, [value]);

  useEffect(() => {
    const parsed = hexToHsv(value);
    if (parsed && parsed.s > 0.001) setHue(parsed.h);
  }, [value]);

  useEffect(() => {
    if (!open) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    // The edited colour itself does not change the square; only its hue and
    // the fixed colours on the other side of each rule do. `referenceColors`
    // keeps a redraw tied to another palette change.
    drawSquare(canvas, hue, requirementsRef.current);
  }, [open, hue, referenceColors]);

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (itemRef.current?.contains(target)) return;
      closeRef.current(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      // The hex field handles Escape itself: it restores the committed value
      // and blurs. Closing here would swallow that event if it stopped.
      const typingHex = document.activeElement?.id === inputId;
      const inside = popoverRef.current?.contains(document.activeElement);
      closeRef.current(false);
      if (!typingHex) event.stopPropagation();
      if (inside) swatchRef.current?.focus();
    };
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown, true);
    };
  }, [open, inputId]);

  const current = hexToHsv(value) ?? { h: hue, s: 0, v: 0 };
  const passing = requirements.every((requirement) => meetsRequirement(value, requirement));
  const worst = worstRequirement(value, requirements);
  const worstRatio = worst ? contrastForRequirement(value, worst) : null;

  const pick = (event: React.PointerEvent<HTMLDivElement>) => {
    const rect = squareRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0 || rect.height === 0) return;
    const saturation = clamp01((event.clientX - rect.left) / rect.width);
    const targetValue = clamp01(1 - (event.clientY - rect.top) / rect.height);
    onChange(hsvToHex(hue, saturation, targetValue));
  };

  const adjust = (next: Partial<Hsv>) => {
    onChange(hsvToHex(hue, next.s ?? current.s, next.v ?? current.v));
  };

  return (
    <div
      ref={itemRef}
      className="theme-color-item"
      onBlur={(event) => {
        if (!open) return;
        const next = event.relatedTarget;
        if (next instanceof Node && itemRef.current?.contains(next)) return;
        onOpenChange(false);
      }}
    >
      <p className="theme-color-name">{label}</p>
      <div className="theme-color-control">
        <button
          ref={swatchRef}
          type="button"
          className="theme-color-swatch"
          style={{ backgroundColor: value }}
          aria-label={`Open ${label} color picker`}
          aria-expanded={open}
          aria-controls={open ? popoverId : undefined}
          title={`Open ${label} color picker`}
          onClick={() => onOpenChange(!open)}
        />
        <input
          id={inputId}
          type="text"
          className={`theme-color-hex${highlighted ? " is-picker-target" : ""}`}
          aria-label={`${label} color`}
          value={draft}
          maxLength={7}
          spellCheck={false}
          autoComplete="off"
          onChange={(event) => {
            setDraft(event.target.value);
            const parsed = hexInput(event.target.value);
            if (parsed) onChange(parsed);
          }}
          onBlur={() => {
            const parsed = hexInput(draft);
            if (parsed) {
              setDraft(parsed);
              onChange(parsed);
            } else setDraft(value);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
            if (event.key === "Escape") {
              setDraft(value);
              event.currentTarget.blur();
            }
          }}
        />
      </div>
      {open && (
        <div
          ref={popoverRef}
          id={popoverId}
          className="theme-color-popover"
          role="group"
          aria-label={`${label} color picker`}
        >
          <div
            ref={squareRef}
            className="theme-color-square"
            tabIndex={0}
            role="group"
            aria-label={`${label} saturation and brightness. Use the arrow keys or the hex field.`}
            aria-describedby={guidanceId}
            onPointerDown={(event) => {
              event.currentTarget.setPointerCapture(event.pointerId);
              event.currentTarget.focus();
              pick(event);
            }}
            onPointerMove={(event) => {
              if (event.buttons === 1) pick(event);
            }}
            onKeyDown={(event) => {
              const step = event.shiftKey ? 0.1 : 0.02;
              if (event.key === "ArrowLeft") adjust({ s: current.s - step });
              else if (event.key === "ArrowRight") adjust({ s: current.s + step });
              else if (event.key === "ArrowUp") adjust({ v: current.v + step });
              else if (event.key === "ArrowDown") adjust({ v: current.v - step });
              else return;
              event.preventDefault();
            }}
          >
            <canvas ref={canvasRef} className="theme-color-canvas" aria-hidden="true" />
            <span
              className="theme-color-marker"
              style={{ left: `${current.s * 100}%`, top: `${(1 - current.v) * 100}%` }}
              aria-hidden="true"
            />
          </div>
          <SliderInput
            className="theme-color-hue"
            aria-label={`${label} hue`}
            aria-valuetext={`${Math.round(hue)} degrees`}
            min={0}
            max={359}
            step={1}
            value={Math.min(359, Math.round(hue))}
            onValue={(next) => {
              setHue(next);
              if (current.s > 0.001) onChange(hsvToHex(next, current.s, current.v));
            }}
          />
          <div className="theme-color-guidance" id={guidanceId}>
            {requirements.length === 0 ? (
              <p>No text is drawn on this color, so it has no contrast line.</p>
            ) : (
              <>
                <p className="theme-color-contrast-key">
                  <i aria-hidden="true" />
                  Contrast line: the dimmed side misses the guidance.
                </p>
                <p className="theme-color-contrast-now">
                  {passing || !worst || worstRatio === null
                    ? "This color meets the guidance."
                    : `Now ${worstRatio.toFixed(1)}:1, needs ${worst.minimum}:1.`}
                </p>
                <p className="theme-color-contrast-pair">
                  {passing || !worst
                    ? "All checked pairs pass."
                    : `${THEME_COLORS[worst.fore]} on ${THEME_COLORS[worst.back].toLowerCase()}`}
                </p>
                <p className="theme-color-contrast-hint">
                  {passing ? "Keep it where the guidance passes." : directionHint(current, requirements)}
                </p>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
