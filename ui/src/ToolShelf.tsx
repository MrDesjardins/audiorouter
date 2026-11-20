import { useState, type DragEvent, type ReactNode, type RefObject } from "react";
import { Icon, libraryEntryIcon, type IconName } from "./icons";
import { endLibraryDrag, startLibraryDrag } from "./libraryDrag";
import { toolHelp, toolLabel, type LibraryEntry } from "./library";
import { useMessages } from "./i18n/useMessages";
import { ToolGuideLink } from "./ToolGuideLink";
import {
  kindOrder,
  libraryEntryKind,
  needsSetup,
  readOpenGroups,
  readPins,
  readRecents,
  rememberRecent,
  SHELF_GROUPS,
  shelfRows,
  togglePin,
  TOOL_GROUP_OF,
  writeOpenGroups,
  writePins,
  writeRecents,
  type ShelfGroupId,
} from "./toolGroups";

type ShelfTool = {
  id: string;
  label: string;
  help: string;
  icon: IconName;
  group: ShelfGroupId;
  order: number;
  guideKind: string;
  guideName: string;
  disabled: boolean;
  /** `fromKeyboard`: Enter or Space, so focus can follow the new card. */
  activate: (fromKeyboard: boolean) => void;
  dragKind?: NonNullable<LibraryEntry["kind"]>;
  setup?: { reason: string; opensCables: boolean };
};

let escapeReturn: ((event: KeyboardEvent) => void) | null = null;

/**
 * After Enter adds a tool, focus sits on the new card so the arrow keys place
 * it. Escape on that card hands focus back to the tool that added it, so a
 * keyboard user can carry on through the toolbox. The card stays (Ctrl+Z or
 * Delete removes it).
 */
function returnToToolOnEscape(pressed: HTMLButtonElement, toolId: string) {
  if (escapeReturn) document.removeEventListener("keydown", escapeReturn, true);
  const listener = (event: KeyboardEvent) => {
    if (event.key !== "Escape") return;
    document.removeEventListener("keydown", listener, true);
    if (escapeReturn === listener) escapeReturn = null;
    const active = document.activeElement;
    if (!(active instanceof Element) || !active.closest(".react-flow__node")) return;
    // The pressed button, or its copy if a re-render replaced it.
    const button = pressed.isConnected
      ? pressed
      : document.querySelector<HTMLButtonElement>(`[data-tool-id="${CSS.escape(toolId)}"]`);
    if (!button) return;
    event.preventDefault();
    button.focus();
  };
  escapeReturn = listener;
  document.addEventListener("keydown", listener, true);
}

export function ToolShelf({
  tools,
  connected,
  search,
  searchRef,
  pluginsContent,
  onAdd,
  onAddCable,
  onAddGroup,
  onApplicationPicker,
  onClearSearch,
  onOpenVirtualCables,
}: {
  tools: LibraryEntry[];
  connected: boolean;
  search: string;
  searchRef: RefObject<HTMLInputElement | null>;
  pluginsContent?: ReactNode;
  onAdd: (kind: NonNullable<LibraryEntry["kind"]>, options?: { focus?: boolean }) => void;
  onAddCable?: (direction: "renderSource" | "captureSink") => void;
  onAddGroup?: () => void;
  onApplicationPicker: () => void;
  onClearSearch: () => void;
  onOpenVirtualCables: () => void;
}) {
  // Recents are read once per visit: adding a tool must not reorder the rows under the
  // pointer (UI-17), so they refresh when someone comes back to the tab. Pins show at
  // once: pressing a star is a deliberate request to change the top rows.
  const [shownRecents] = useState(readRecents);
  const [pins, setPins] = useState(readPins);
  const [openGroups, setOpenGroups] = useState(readOpenGroups);
  const [describedId, setDescribedId] = useState<string | null>(null);
  const tr = useMessages();
  const query = search.trim().toLocaleLowerCase();

  const all = (() => {
    const matches = (text: string) => !query || text.toLocaleLowerCase().includes(query);
    const fromLibrary = tools.map((entry): ShelfTool => {
      const kind = libraryEntryKind(entry);
      const setup = needsSetup(entry);
      const { kind: addKind, virtualKind } = entry;
      return {
        id: entry.id,
        label: toolLabel(entry),
        help: entry.unavailableReason ?? toolHelp(entry),
        icon: libraryEntryIcon(entry),
        group: setup ? "setup" : TOOL_GROUP_OF[kind],
        order: kindOrder(kind),
        guideKind: kind,
        guideName: entry.label,
        disabled: !connected || setup || (!addKind && !onAddCable),
        activate: (fromKeyboard) => {
          if (addKind) onAdd(addKind, { focus: fromKeyboard });
          else if (virtualKind) onAddCable?.(virtualKind === "virtualRenderSource" ? "renderSource" : "captureSink");
        },
        dragKind: addKind,
        setup: setup
          ? { reason: entry.unavailableReason ?? entry.category, opensCables: virtualKind !== undefined }
          : undefined,
      };
    });
    const application: ShelfTool = {
      id: "application-source",
      label: tr("toolShelf.application"),
      help: tr("toolShelf.applicationHelp"),
      icon: "app-window",
      group: "input",
      order: kindOrder("applicationCapture"),
      guideKind: "applicationCapture",
      guideName: "Application Capture",
      disabled: !connected,
      activate: onApplicationPicker,
    };
    const group: ShelfTool = {
      id: "group",
      label: tr("toolShelf.group"),
      help: tr("toolShelf.groupHelp"),
      icon: "square-dashed",
      group: "canvas",
      order: 0,
      guideKind: "group",
      guideName: "Group",
      disabled: false,
      activate: () => onAddGroup?.(),
    };
    return [...fromLibrary, ...[application, group].filter((item) => matches(`${item.label} ${item.help}`))].sort(
      (left, right) => left.order - right.order,
    );
  })();

  const byId = new Map(all.map((tool) => [tool.id, tool]));
  const described = describedId === null ? undefined : byId.get(describedId);
  const usable = new Set(all.filter((tool) => !tool.setup).map((tool) => tool.id));
  const topRows = shelfRows(pins, shownRecents, usable);
  const hasHistory = pins.length + shownRecents.length > 0;

  const remember = (id: string) => writeRecents(rememberRecent(readRecents(), id));
  const toggleGroup = (id: ShelfGroupId) =>
    setOpenGroups((current) => {
      const next = current.includes(id) ? current.filter((item) => item !== id) : [...current, id];
      writeOpenGroups(next);
      return next;
    });

  const renderRow = (tool: ShelfTool, buttonClass: string) => {
    const pinned = pins.includes(tool.id);
    return (
      <div
        className={`tool-library-entry tool-row${pinned ? " is-pinned" : ""}`}
        key={tool.id}
        onMouseEnter={() => setDescribedId(tool.id)}
        onFocus={() => setDescribedId(tool.id)}
      >
        <button
          type="button"
          className={buttonClass}
          draggable={connected && Boolean(tool.dragKind)}
          disabled={tool.disabled}
          aria-description={tool.help}
          onDragStart={(event) => {
            if (tool.dragKind) startLibraryDrag(event, tool.dragKind, tool.guideName);
          }}
          onDragEnd={(event: DragEvent) => {
            endLibraryDrag();
            if (event.dataTransfer.dropEffect !== "none") remember(tool.id);
          }}
          data-tool-id={tool.id}
          onClick={(event) => {
            // A keyboard-activated click has no pointer clicks (detail 0).
            const fromKeyboard = event.detail === 0;
            tool.activate(fromKeyboard);
            if (fromKeyboard) returnToToolOnEscape(event.currentTarget, tool.id);
            remember(tool.id);
          }}
        >
          <span className="tool-card-icon" aria-hidden="true">
            <Icon name={tool.icon} size={18} />
          </span>
          <strong>{tool.label}</strong>
        </button>
        {!tool.setup && (
          <button
            type="button"
            className="tool-pin"
            aria-label={tr(pinned ? "toolShelf.unpin" : "toolShelf.pin", { name: tool.label })}
            aria-pressed={pinned}
            title={tr(pinned ? "toolShelf.unpinTitle" : "toolShelf.pinTitle")}
            onClick={() =>
              setPins((current) => {
                const next = togglePin(current, tool.id);
                writePins(next);
                return next;
              })
            }
          >
            <Icon name="star" size={14} />
          </button>
        )}
        <ToolGuideLink kind={tool.guideKind} compact toolName={tool.label} />
      </div>
    );
  };

  const renderSetupRow = (tool: ShelfTool) => (
    <div className="tool-setup" key={tool.id}>
      {renderRow(tool, "tool-card")}
      <small>{tool.setup?.reason}</small>
      {tool.setup?.opensCables && (
        <button type="button" className="secondary" onClick={onOpenVirtualCables}>
          {tr("toolShelf.openCables")}
        </button>
      )}
    </div>
  );

  const searching = query.length > 0;
  const nothingFound = searching && all.length === 0;
  return (
    <div className="tool-shelf">
      {nothingFound && (
        <div className="tool-search-empty">
          <p role="status">{tr("toolShelf.empty")}</p>
          <button
            type="button"
            className="secondary"
            onClick={() => {
              onClearSearch();
              searchRef.current?.focus();
            }}
          >
            {tr("toolShelf.clear")}
          </button>
        </div>
      )}
      {!searching && (
        <section className="tool-group tool-group-top" aria-labelledby="tool-group-top-heading">
          <div className="tool-group-heading">
            <h3 id="tool-group-top-heading">{tr(hasHistory ? "toolShelf.recent" : "toolShelf.common")}</h3>
            <small>{tr(hasHistory ? "toolShelf.pinHint" : "toolShelf.recentHint")}</small>
          </div>
          {topRows.flatMap((id) => {
            const tool = byId.get(id);
            return tool ? [renderRow(tool, "tool-card-recent")] : [];
          })}
        </section>
      )}
      {SHELF_GROUPS.map(({ id }) => {
        const members = all.filter((tool) => tool.group === id);
        const isPlugins = id === "plugins";
        if (searching && members.length === 0 && !isPlugins) return null;
        const open = searching || openGroups.includes(id);
        const panelId = `tool-group-panel-${id}`;
        return (
          <section className="tool-group" key={id}>
            <h3 className="tool-group-title">
              <button
                type="button"
                className="tool-group-toggle"
                aria-expanded={open}
                aria-controls={panelId}
                disabled={searching}
                onClick={() => toggleGroup(id)}
              >
                <Icon name={open ? "chevron-down" : "chevron-right"} size={14} />
                <span>{tr(`toolShelf.${id}`)}</span>
                {!isPlugins && <span className="tool-group-count">{members.length}</span>}
              </button>
            </h3>
            {open && (
              <div id={panelId}>
                {members.map((tool) => (tool.setup ? renderSetupRow(tool) : renderRow(tool, "tool-card")))}
                {isPlugins && pluginsContent}
              </div>
            )}
          </section>
        );
      })}
      <div className="tool-shelf-description" aria-hidden="true">
        {described ? (
          <>
            <b>{described.label}</b>
            <span>{described.help}</span>
          </>
        ) : (
          <span>{tr("toolShelf.preview")}</span>
        )}
      </div>
    </div>
  );
}
