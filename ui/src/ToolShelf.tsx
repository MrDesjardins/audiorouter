import { useMemo, useState, type DragEvent, type ReactNode, type RefObject } from "react";
import { Icon, libraryEntryIcon, type IconName } from "./icons";
import { endLibraryDrag, startLibraryDrag } from "./libraryDrag";
import { TOOL_HELP, type LibraryEntry } from "./library";
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
  activate: () => void;
  dragKind?: NonNullable<LibraryEntry["kind"]>;
  setup?: { reason: string; opensCables: boolean };
};

const APPLICATION_HELP = "Capture audio from a running app into this route.";
const GROUP_HELP = "A named visual background. It carries no audio.";
const DESCRIPTION_PLACEHOLDER = "Point at a tool to see what it does. Click to add it, or drag it onto the canvas.";

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
  onAdd: (kind: NonNullable<LibraryEntry["kind"]>) => void;
  onAddCable?: (direction: "renderSource" | "captureSink") => void;
  onAddGroup?: () => void;
  onApplicationPicker: () => void;
  onClearSearch: () => void;
  onOpenVirtualCables: () => void;
}) {
  // Read once per visit: this panel unmounts with the tab, so the top rows refresh when
  // someone comes back and never move under the pointer (UI-17).
  const [shownRecents] = useState(readRecents);
  const [shownPins] = useState(readPins);
  const [pins, setPins] = useState(shownPins);
  const [openGroups, setOpenGroups] = useState(readOpenGroups);
  const [described, setDescribed] = useState<ShelfTool | null>(null);
  const query = search.trim().toLocaleLowerCase();

  const all = useMemo<ShelfTool[]>(() => {
    const matches = (text: string) => !query || text.toLocaleLowerCase().includes(query);
    const fromLibrary = tools.map((entry): ShelfTool => {
      const kind = libraryEntryKind(entry);
      const setup = needsSetup(entry);
      const { kind: addKind, virtualKind } = entry;
      return {
        id: entry.id,
        label: entry.label,
        help: entry.unavailableReason ?? entry.note ?? TOOL_HELP[entry.id] ?? entry.category,
        icon: libraryEntryIcon(entry),
        group: setup ? "setup" : TOOL_GROUP_OF[kind],
        order: kindOrder(kind),
        guideKind: kind,
        guideName: entry.label,
        disabled: !connected || setup || (!addKind && !onAddCable),
        activate: () => {
          if (addKind) onAdd(addKind);
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
      label: "Application source",
      help: APPLICATION_HELP,
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
      label: "Group",
      help: GROUP_HELP,
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
  }, [tools, connected, query, onAdd, onAddCable, onAddGroup, onApplicationPicker]);

  const byId = new Map(all.map((tool) => [tool.id, tool]));
  const usable = new Set(all.filter((tool) => !tool.setup).map((tool) => tool.id));
  const topRows = shelfRows(shownPins, shownRecents, usable);
  const hasHistory = shownPins.length + shownRecents.length > 0;

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
        onMouseEnter={() => setDescribed(tool)}
        onFocus={() => setDescribed(tool)}
      >
        <button
          type="button"
          className={buttonClass}
          draggable={connected && Boolean(tool.dragKind)}
          disabled={tool.disabled}
          aria-description={tool.help}
          onDragStart={(event) => {
            if (tool.dragKind) startLibraryDrag(event, tool.dragKind, tool.label);
          }}
          onDragEnd={(event: DragEvent) => {
            endLibraryDrag();
            if (event.dataTransfer.dropEffect !== "none") remember(tool.id);
          }}
          onClick={() => {
            tool.activate();
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
            aria-label={pinned ? `Unpin ${tool.label}` : `Pin ${tool.label}`}
            aria-pressed={pinned}
            title={pinned ? "Unpin" : "Pin to the top"}
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
        <ToolGuideLink kind={tool.guideKind} compact toolName={tool.guideName} />
      </div>
    );
  };

  const renderSetupRow = (tool: ShelfTool) => (
    <div className="tool-setup" key={tool.id}>
      {renderRow(tool, "tool-card")}
      <small>{tool.setup?.reason}</small>
      {tool.setup?.opensCables && (
        <button type="button" className="secondary" onClick={onOpenVirtualCables}>
          Open Virtual Cables
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
          <p role="status">No built-in tools match your search. Try a name such as Gain or Recorder.</p>
          <button
            type="button"
            className="secondary"
            onClick={() => {
              onClearSearch();
              searchRef.current?.focus();
            }}
          >
            Clear tool search
          </button>
        </div>
      )}
      {!searching && (
        <section className="tool-group tool-group-top" aria-labelledby="tool-group-top-heading">
          <div className="tool-group-heading">
            <h3 id="tool-group-top-heading">{hasHistory ? "Recent" : "Common tools"}</h3>
            <small>{hasHistory ? "Pin a tool to keep it here" : "Your recent tools replace these"}</small>
          </div>
          {topRows.flatMap((id) => {
            const tool = byId.get(id);
            return tool ? [renderRow(tool, "tool-card-recent")] : [];
          })}
        </section>
      )}
      {SHELF_GROUPS.map(({ id, label }) => {
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
                <span>{label}</span>
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
          <span>{DESCRIPTION_PLACEHOLDER}</span>
        )}
      </div>
    </div>
  );
}
