/** A small duck (head, beak, eye, body) in the icon colour; no font has one. */
export function DuckGlyph({ className, size = "1.3em" }: { className?: string; size?: string }) {
  return <svg viewBox="0 0 24 24" width={size} height={size} fill="currentColor" aria-hidden="true" className={className}>
    <path fillRule="evenodd" d="M9.2 3.6a3.6 3.6 0 0 0-3.1 5.4c-.6.4-1 1-1.1 1.7-1.6.2-2.9 1.4-2.9 3.1C2.1 16.9 4.9 19.5 9 19.5h5.4c3.9 0 6.9-2.5 6.9-5.6 0-1.8-1-3.2-2.4-3.9l-2.4 1.3a6 6 0 0 0-3-.9 3.6 3.6 0 0 0-.7-5.6 3.6 3.6 0 0 0-3.6-1.2Zm.6 2.3a.9.9 0 1 1 0 1.8.9.9 0 0 1 0-1.8Z" />
    <path d="M12.1 6.1 16.9 7.4l-4.6 2.1Z" />
  </svg>;
}
