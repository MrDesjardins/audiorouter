---
applies_to: ui/**/*.tsx, ui/**/*.css
---

# Live data must not shift the layout (UI-17)
Live values, suggestions, status text, and pills must never make the UI move. Render each such slot at a reserved size, show a placeholder (with a disabled action) while waiting, keep the last good value through brief gaps, and fix the width of changing labels. Only a deliberate user action may change layout.

Good:
```tsx
<span className="status-slot" aria-live="polite">
  {latency ?? "—"}
</span>
```
```css
.status-slot { display: inline-block; min-width: 8ch; font-variant-numeric: tabular-nums; }
```

Bad:
```tsx
{latency !== undefined && <span className="status">{latency} ms</span>}
```

# Form fields use the app-wide field style
Text inputs, selects, textareas, and file pickers must inherit the shared field style in `ui/src/styles.css`. Do not restyle fields per panel, and do not add gradients, inset shadows, or bevels to fields; only buttons are bevelled.

Good:
```tsx
<input type="text" value={name} onChange={onNameChange} />
```

Bad:
```css
.eq-panel input[type="text"] {
  border-radius: 3px;
  background: linear-gradient(#222, #333);
  box-shadow: inset 0 1px 2px #000;
}
```

# Field captions stack above a full-width field
A field's caption goes above the field, and the field takes the full width. Do not let a label, a field, and buttons flow inline on one row.

Good:
```tsx
<label>
  <span>Device name</span>
  <input type="text" value={name} onChange={onNameChange} />
</label>
```

Bad:
```tsx
<div>
  Device name <input type="text" value={name} onChange={onNameChange} /> <button>Save</button>
</div>
```
