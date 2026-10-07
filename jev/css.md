---
applies_to: **/*.css
---

# Avoid !important

`!important` should not be used to override specificity conflicts; fix the
underlying specificity or cascade order instead. It makes styles hard to
override later and is a common source of CSS debugging pain.

Good:
```css
.button--primary {
  background-color: blue;
}
```

Bad:
```css
.button {
  background-color: blue !important;
}
```

# Use relative units for font sizes

Font sizes should use `rem` or `em`, not `px`, so text scales with the
user's browser/OS accessibility settings.

Good:
```css
.body-text {
  font-size: 1rem;
}
```

Bad:
```css
.body-text {
  font-size: 16px;
}
```

# Avoid overly specific selectors

A selector should not chain more than 2-3 levels of nesting or combine ids
with multiple classes; deep selectors are fragile and hard to override.

Good:
```css
.card__title {
  font-weight: 600;
}
```

Bad:
```css
div#page .content .card > ul li a.card__title {
  font-weight: 600;
}
```

# No hardcoded colors outside design tokens

Color values should reference a shared variable/custom property rather than
a hardcoded hex/rgb value repeated across the codebase.

Good:
```css
.alert {
  color: var(--color-danger);
}
```

Bad:
```css
.alert {
  color: #d32f2f;
}
```

# Vendor prefixes must not be added by hand

Do not hand-write vendor-prefixed properties (e.g. `-webkit-`, `-moz-`);
rely on a build-time autoprefixer instead, so prefixes stay correct as
browser support changes.

Good:
```css
.box {
  display: flex;
}
```

Bad:
```css
.box {
  display: -webkit-flex;
  display: flex;
}
```

# Avoid ID selectors for styling
Style with classes, not `#id` selectors; ids should be reserved for JS hooks/anchors, not CSS specificity.

Good:
```css
.submit-button { color: blue; }
```

Bad:
```css
#submit-button { color: blue; }
```

# Do not use inline styles in markup
Styling should live in CSS files/classes, not `style="..."` attributes in HTML/JSX.

Good:
```css
<div class="banner"></div>
```

Bad:
```css
<div style="color: red;"></div>
```

# Prefer flexbox or grid over floats for layout
Use `display: flex` or `display: grid` for layout instead of `float`-based techniques.

Good:
```css
.row { display: flex; }
```

Bad:
```css
.row .col { float: left; }
```

# Avoid nesting selectors more than 3 levels deep
Deeply nested selectors are fragile and hard to override; flatten with BEM-style class names instead.

Good:
```css
.card__title { font-weight: 600; }
```

Bad:
```css
.page .content .card .header .title { font-weight: 600; }
```

# Prefer CSS custom properties for themeable values
Values that vary by theme (colors, spacing scale) should be CSS custom properties, not hardcoded literals.

Good:
```css
color: var(--color-primary);
```

Bad:
```css
color: #1a73e8;
```

# Avoid the universal selector in performance-sensitive rules
Do not use `*` in selectors applied broadly across large pages; it forces the browser to match every element.

Good:
```css
.container > * { margin: 0; }
```

Bad:
```css
* { margin: 0; }
```

# Do not use px for line-height
`line-height` should be a unitless number, not a fixed pixel value, so it scales with font-size.

Good:
```css
line-height: 1.5;
```

Bad:
```css
line-height: 24px;
```

# Prefer shorthand properties when setting all sides equally
Use `margin: 8px;` instead of setting `margin-top`, `margin-right`, `margin-bottom`, `margin-left` individually to the same value.

Good:
```css
margin: 8px;
```

Bad:
```css
margin-top: 8px;
margin-right: 8px;
margin-bottom: 8px;
margin-left: 8px;
```

# Avoid duplicate selectors within the same file
The same selector should not be declared twice in one file with different rules; merge them.

Good:
```css
.btn {
  color: blue;
  padding: 8px;
}
```

Bad:
```css
.btn { color: blue; }
.btn { padding: 8px; }
```

# Do not use fixed pixel widths on responsive containers
A container meant to be responsive should use percentage, `fr`, or `minmax()` widths, not a fixed `px` width.

Good:
```css
width: 100%;
max-width: 640px;
```

Bad:
```css
width: 640px;
```

# Prefer rem for spacing over hardcoded px
Margins and paddings should generally use `rem` so spacing scales with the root font size.

Good:
```css
padding: 1rem;
```

Bad:
```css
padding: 16px;
```

# Avoid z-index values without a documented stacking scale
A `z-index` value should come from a small, documented scale (e.g. 10/20/30), not an arbitrary large number.

Good:
```css
z-index: var(--z-modal); /* 30 */
```

Bad:
```css
z-index: 9999;
```

# Avoid transition: all
`transition: all` should be avoided; list the specific properties being transitioned for predictability and performance.

Good:
```css
transition: opacity 0.2s, transform 0.2s;
```

Bad:
```css
transition: all 0.2s;
```

# Do not hide focusable content with overflow: hidden without care
`overflow: hidden` on a scrollable focus container must not trap keyboard focus on hidden content.

Good:
```css
.menu { overflow: visible; }
```

Bad:
```css
.menu { overflow: hidden; } /* tab-focused items get clipped */
```

# Prefer aspect-ratio over the padding-hack for responsive media
Use the `aspect-ratio` property instead of the padding-percentage trick for maintaining media aspect ratio.

Good:
```css
.video { aspect-ratio: 16 / 9; }
```

Bad:
```css
.video { position: relative; padding-bottom: 56.25%; }
```

# Avoid magic breakpoint values
Media query breakpoints should reference shared variables, not one-off pixel values scattered across files.

Good:
```css
@media (min-width: $breakpoint-md) { ... }
```

Bad:
```css
@media (min-width: 782px) { ... }
```

# Do not leave empty rule blocks
A selector with an empty `{}` body should be removed.

Good:
```css
.card { padding: 8px; }
```

Bad:
```css
.card {}
```

# Prefer logical properties over physical ones for i18n
Use `margin-inline-start` instead of `margin-left` so layouts correctly mirror for right-to-left languages.

Good:
```css
margin-inline-start: 8px;
```

Bad:
```css
margin-left: 8px;
```

# Do not use color alone to convey state
State (error, success, disabled) must be conveyed with more than color alone, so it's accessible to colorblind users.

Good:
```css
.error { color: red; } .error::before { content: "⚠ "; }
```

Bad:
```css
.error { color: red; }
```

# Prefer CSS Grid named areas for complex layouts
A layout with several named regions should use `grid-template-areas` rather than deeply nested flex containers.

Good:
```css
grid-template-areas: "header header" "sidebar content";
```

Bad:
```css
.layout .sidebar { float: left; } .layout .content { float: right; }
```

# Avoid inconsistent unitless-zero usage
Be consistent about omitting units on zero values (`0` vs `0px`) within the same file.

Good:
```css
margin: 0;
padding: 0;
```

Bad:
```css
margin: 0px;
padding: 0;
```

# Do not duplicate the same color value repeatedly
A color value used more than twice should be extracted into a shared variable/custom property.

Good:
```css
border-color: var(--color-border);
```

Bad:
```css
border-color: #e0e0e0;
```

# Prefer removing will-change after the animation completes
`will-change` should be applied only while an animation is active and removed afterward, not left on permanently.

Good:
```css
.el.animating { will-change: transform; }
```

Bad:
```css
.el { will-change: transform; } /* left on permanently */
```

# Avoid box-shadow spreads that overflow their container
A `box-shadow` spread value should not visually break out of a container with `overflow: hidden` in an unintended way.

Good:
```css
box-shadow: 0 2px 4px rgba(0,0,0,0.2);
```

Bad:
```css
box-shadow: 0 0 0 50px red; /* breaks out of a clipped container */
```

# Do not use display: none to hide content that should remain accessible
Content still meant to be available to assistive tech should use visually-hidden techniques, not `display: none`.

Good:
```css
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; }
```

Bad:
```css
.sr-only { display: none; }
```

# Prefer consistent quote style for CSS strings
Use one quote style (single or double) consistently for string values like `content` or `url()`.

Good:
```css
content: "→";
```

Bad:
```css
content: '→';
content: "←";
```

# Avoid redundant tag+class selector chains
Do not write `div.container` when `.container` alone is sufficient and less brittle.

Good:
```css
.container { ... }
```

Bad:
```css
div.container { ... }
```

# Do not hardcode font stacks across many files
A font-family stack should be defined once as a variable and reused, not repeated inline everywhere.

Good:
```css
font-family: var(--font-body);
```

Bad:
```css
font-family: -apple-system, "Segoe UI", sans-serif;
```

# Avoid excessive use of !important as a specificity workaround
Repeated `!important` usage across a file is a sign the selector architecture needs restructuring, not another override.

Good:
```css
.btn--primary { color: blue; }
```

Bad:
```css
.btn { color: blue !important; }
```

# Prefer prefers-reduced-motion support for animations
Non-essential animations should respect the `prefers-reduced-motion` media feature.

Good:
```css
@media (prefers-reduced-motion: no-preference) {
  .el { transition: transform 0.3s; }
}
```

Bad:
```css
.el { transition: transform 0.3s; }
```

# Margin collapsing between adjacent block elements
Vertical margins between adjacent (or parent/child) block-level elements can collapse into a single margin instead of adding together, which surprises people expecting the sum.

Good:
```css
.card + .card {
  margin-top: 16px; /* only one side sets it, avoiding the collapse ambiguity */
}
```

Bad:
```css
.card {
  margin-top: 16px;
  margin-bottom: 16px; /* adjacent cards collapse to 16px gap, not 32px */
}
```

# position: absolute is relative to the nearest positioned ancestor
An absolutely positioned element is placed relative to its nearest ancestor with a `position` other than `static`, not the viewport — a missing `position: relative` on the intended container sends it somewhere else entirely.

Good:
```css
.card {
  position: relative;
}
.card .badge {
  position: absolute;
  top: 0;
  right: 0;
}
```

Bad:
```css
.card .badge {
  position: absolute; /* .card has no position set, so this escapes further up the tree */
  top: 0;
  right: 0;
}
```

# z-index has no effect on statically positioned elements
`z-index` only applies to elements with a `position` value other than `static` (or that are flex/grid items); setting it on a normally-positioned element does nothing.

Good:
```css
.modal {
  position: relative;
  z-index: 10;
}
```

Bad:
```css
.modal {
  z-index: 10; /* no effect: position defaults to static */
}
```

# Percentage widths resolve against the containing block
A percentage width/height is relative to the containing block's size, not the viewport — if the containing block has no defined size, the percentage may resolve to 0 or behave unexpectedly.

Good:
```css
.container {
  width: 800px;
}
.child {
  width: 50%; /* 400px, relative to .container */
}
```

Bad:
```css
.child {
  width: 50%; /* relative to an ancestor whose own width is also auto/unset */
}
```

# Flex/grid items have an implicit min-width: auto that can prevent shrinking
Flex and grid items default to `min-width: auto` (based on content), which can stop them from shrinking below their content size even with `flex-shrink` set — causing overflow.

Good:
```css
.flex-item {
  min-width: 0;
  flex-shrink: 1;
}
```

Bad:
```css
.flex-item {
  flex-shrink: 1; /* still won't shrink below its content's intrinsic width */
}
```

# Transitions do not trigger across display: none
A CSS transition does not animate when a property changes at the same time an element goes from `display: none` to visible (or vice versa); the element must already be rendered for the transition to apply.

Good:
```css
.modal {
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.2s;
}
.modal.open {
  opacity: 1;
  visibility: visible;
}
```

Bad:
```css
.modal {
  display: none;
  transition: opacity 0.2s;
}
.modal.open {
  display: block;
  opacity: 1; /* no transition plays */
}
```

# Ensure focus states are visible, not just hover states
Interactive elements need a visible `:focus` (or `:focus-visible`) style distinct from `:hover`; keyboard users never trigger `:hover` and rely entirely on the focus indicator to know where they are.

Good:
```css
.btn:hover, .btn:focus-visible {
  outline: 2px solid var(--color-focus);
}
```

Bad:
```css
.btn:hover {
  outline: 2px solid blue;
} /* keyboard-only users never see any focus indicator */
```

# Do not remove outline without providing a replacement focus style
`outline: none` removes the browser's default focus indicator; it must always be paired with a custom, clearly visible focus style, or keyboard navigation becomes unusable.

Good:
```css
button:focus {
  outline: none;
  box-shadow: 0 0 0 3px var(--color-focus);
}
```

Bad:
```css
button:focus {
  outline: none; /* no replacement: focus becomes invisible */
}
```

# Use a consistent spacing scale instead of ad hoc values
Spacing values (margin, padding, gap) should come from a small shared scale (e.g. multiples of 4px/8px), not arbitrary one-off numbers scattered across the codebase, for visual consistency.

Good:
```css
padding: var(--space-2); /* 8px, from the shared scale */
```

Bad:
```css
padding: 7px; /* arbitrary value not on the shared spacing scale */
```

# Prefer transform/opacity for animations over layout-triggering properties
Animating `top`/`left`/`width`/`height` forces the browser to recompute layout on every frame; animating `transform`/`opacity` instead can run on the compositor, avoiding layout thrash.

Good:
```css
.modal {
  transition: transform 0.2s, opacity 0.2s;
}
.modal.open {
  transform: translateY(0);
  opacity: 1;
}
```

Bad:
```css
.modal {
  transition: top 0.2s;
}
.modal.open {
  top: 0; /* triggers layout on every animation frame */
}
```

# Do not rely on color alone for interactive affordances
A clickable element distinguished only by a slightly different color (with no underline, icon, or other cue) is hard to identify for colorblind users and easy to miss visually.

Good:
```css
a {
  color: var(--color-link);
  text-decoration: underline;
}
```

Bad:
```css
a {
  color: var(--color-link); /* no underline or other non-color cue that it's clickable */
}
```

# Avoid disabling text selection broadly
`user-select: none` applied broadly (e.g. on `body` or large containers) prevents users from copying text they legitimately want to select; scope it narrowly to genuinely non-text UI (icons, drag handles).

Good:
```css
.drag-handle {
  user-select: none;
}
```

Bad:
```css
body {
  user-select: none; /* users can no longer copy any text on the page */
}
```
