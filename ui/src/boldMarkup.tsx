import { Fragment, type ReactNode } from "react";

/**
 * Renders a translated message with simple markup: <b>…</b> is emphasis and <i>…</i> marks a
 * literal (a command, address or setting name) shown in code type. The catalog checker only
 * allows br/strong/em/b/i, so <i> stands in for <code>. Nothing else is interpreted.
 */
export function boldMarkup(text: string): ReactNode {
  return text.split(/(<b>.*?<\/b>|<i>.*?<\/i>)/g).map((part, index) => {
    if (part.startsWith("<b>")) return <strong key={index}>{part.slice(3, -4)}</strong>;
    if (part.startsWith("<i>")) return <code key={index}>{part.slice(3, -4)}</code>;
    return <Fragment key={index}>{part}</Fragment>;
  });
}
