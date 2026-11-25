# Fluent-review queue for the French and Simplified Chinese catalogs

Date: 2026-11-24. Status: **not reviewed by a fluent speaker.** The text below
was written by the agent from the existing catalog terms. It passes the
mechanical checks (`npm --prefix ui run i18n:check`); it is not translation or
terminology evidence.

Owner decision recorded 2026-11-24: the plans with only human gates left are
archived, so this file is the reviewer's work queue. A fluent reviewer approves
or corrects the wording before either language is called supported (L10N-08);
the packaged shell, tray and Narrator checks stay separate owner gates.

## How to review

1. Run the app (`npm --prefix ui run dev` or the staged Windows build) and
   switch Settings → Appearance → Language to `Français` or `简体中文`.
2. Enter the screen named in the table below and read the English sentence
   beside its translation. Open each state that shows the message: a refused
   connection, an occupied device, a route that stops when a device is lost, a
   Plug-in scan failure, and the application picker.
3. Compare placeholders: every `{{name}}`, `{{device}}`, `{{bus}}`, `{{scope}}`,
   `{{min}}`, `{{max}}`, `{{taken}}`, `{{revision}}`, `{{label}}`, `{{value}}`
   and `{{unit}}` must survive in the same position of the sentence. Names,
   IDs, paths and `{{unit}}` values are never translated.
4. Check the safety text first (L10N-06): a permission refusal, a privacy/mute
   warning, and a destructive action must keep their meaning and strength.
5. Record a decision per row: accept, correct (paste the new text), or leave
   open with the reason. A correction is made in
   `ui/src/i18n/locales/fr.json` or `zh-Hans.json`, then
   `npm --prefix ui run i18n:generate` and `npm --prefix ui run i18n:check`.

`npm --prefix ui run i18n:audit -- --tsx --by-file` lists the remaining
unlocalized visible candidates (units, product names, paths and preview
fixtures) for a second opinion; it is an inventory, not a translation verdict.

## Text added or changed on 2026-11-24

| Key | English text | Where it appears | Placeholders |
|---|---|---|---|
| `h2.goalTitleMicDiscord` | Mic to Discord | Goal picker: Mic to Discord card | none |
| `h2.goalSumMicDiscord` | Your cleaned voice goes to the cable Discord listens to. | Goal picker: Mic to Discord card | none |
| `workspaceEvents.recoveryFailed` | AudioRouter could not restart the route: the device it uses did not come back with the same identity. Check the device in Properties, then press Play. | Status line during route recovery | none |
| `workspaceEvents.recovered` | The audio device came back, so AudioRouter restarted the route. | Status line during route recovery | none |
| `workspaceEvents.deviceChanged` | An audio device this route uses was disconnected or changed, so audio stopped. AudioRouter restarts the route when the same device is back, for up to two minutes; after that, reconnect it and press Play. | Status line during route recovery | none |
| `workspaceEvents.bridgeTimedOut` | The virtual cable link for {{bus}} timed out, so that audio is silent. Press Stop and Play to restart it. | Status line during route recovery | bus |
| `workspaceEvents.bridgeTimedOutUnnamed` | The virtual cable link for an affected bus timed out, so that audio is silent. Press Stop and Play to restart it. | Status line during route recovery | none |
| `workspaceEvents.bridgeFailed` | The virtual cable link for {{bus}} failed, so that audio is silent. Press Stop and Play to restart it. | Status line during route recovery | bus |
| `workspaceEvents.bridgeFailedUnnamed` | The virtual cable link for an affected bus failed, so that audio is silent. Press Stop and Play to restart it. | Status line during route recovery | none |
| `appShell.goToTool` | Go to {{name}} | Session revision line / tool button | name |
| `appShell.revisionLive` | Revision {{revision}} - edits stay unsaved until you review and save them | Session revision line / tool button | revision |
| `appShell.revisionPreview` | Revision {{revision}} - changes are presentation-only in this preview | Session revision line / tool button | revision |
| `draftErrors.runningAppOnly` | Only a verified running application can be added to the graph | Refusal shown in the status line | none |
| `draftErrors.exePath` | Choose a full Windows path to an .exe file. | Refusal shown in the status line | none |
| `draftErrors.exeFile` | Choose an .exe application file. | Refusal shown in the status line | none |
| `draftErrors.exeNameMismatch` | The executable name does not match its selected path. | Refusal shown in the status line | none |
| `draftErrors.exePathUnavailable` | A full executable path for {{name}} is unavailable; choose its .exe file. | Refusal shown in the status line | name |
| `draftErrors.groupNeedsTwo` | Choose at least two different applications for a group. | Refusal shown in the status line | none |
| `draftErrors.scanResultOnly` | Only a supported x64 scan result can be added to the graph | Refusal shown in the status line | none |
| `draftErrors.selfConnect` | A tool cannot connect to itself | Refusal shown in the status line | none |
| `draftErrors.chooseOutputPort` | Choose an output source port | Refusal shown in the status line | none |
| `draftErrors.chooseInputPort` | Choose an input destination port | Refusal shown in the status line | none |
| `draftErrors.connectionExists` | That connection is already in your unsaved edits | Refusal shown in the status line | none |
| `draftErrors.inputOccupied` | That input already has a connection | Refusal shown in the status line | none |
| `draftErrors.sidechainChanged` | The sidechain connection changed; try again | Refusal shown in the status line | none |
| `draftErrors.outputChanged` | The output connection changed; try again | Refusal shown in the status line | none |
| `draftErrors.destinationChanged` | The destination connection changed; try again | Refusal shown in the status line | none |
| `draftErrors.instrumentInline` | An instrument makes sound; it cannot sit inside a connection. | Refusal shown in the status line | none |
| `draftErrors.mixerRemoval` | Mixer removal requires exactly one incoming and one outgoing connection | Refusal shown in the status line | none |
| `draftErrors.keepOneTool` | A route must keep at least one tool | Refusal shown in the status line | none |
| `draftErrors.toolNameEmpty` | Tool name cannot be empty | Refusal shown in the status line | none |
| `draftErrors.toolNameLong` | Tool name cannot exceed 120 characters | Refusal shown in the status line | none |
| `draftErrors.sessionNameEmpty` | Session name cannot be empty | Refusal shown in the status line | none |
| `draftErrors.sessionNameLong` | Session name cannot exceed 120 characters | Refusal shown in the status line | none |
| `draftErrors.volumeRange` | Volume must be between 0 and 200 % | Refusal shown in the status line | none |
| `draftErrors.mixerVolumeRange` | Mixer input volume must be between 0 and 100 % | Refusal shown in the status line | none |
| `draftErrors.gainRange` | Gain must be between {{min}} and {{max}} dB | Refusal shown in the status line | min, max |
| `signalFlowDialogs.keyboardConnection` | Keyboard connection | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.keyboardConnectionOpen` | Keyboard connection dialog | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.keyboardConnectionClose` | Close keyboard connection dialog | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.keyboardConnectionHelp` | Choose an output and input, then add the connection. Press Escape to close. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.keyboardSourceAria` | Keyboard source output port | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.keyboardDestinationAria` | Keyboard destination input port | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pluginInsertTitle` | Insert a VST2/VST3 plugin into this connection | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pluginAddTitle` | Add a VST2/VST3 plugin | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pluginPickerClose` | Close plugin picker | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pluginHelpInsert` | Scan an absolute directory on this machine for supported x64 VST2/VST3 binaries, then insert one directly into the connection. Press Escape to close. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pluginHelpAdd` | Scan an absolute directory on this machine for supported x64 VST2/VST3 binaries, then add one as a stopped tool. Press Escape to close. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.appPickerTitle` | Add an application capture source | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.appPickerClose` | Close application picker | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.appPickerHelp` | Add a running app or choose its .exe while it is closed. AudioRouter saves the executable identity, not a process ID, then checks for a verified match when you start the route. App groups create separate sources feeding a Mixer; avoid selecting an app together with a helper it launches. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.oneApplication` | One application | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.appGroup` | App group | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.exePathLabel` | Application executable path | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.typePath` | Type the full path to the application's .exe file. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pickerFailed` | Unable to open the file picker. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.browseExe` | Browse for .exe | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pathFirst` | Choose or type an .exe path first. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.addPathToGroup` | Add path to group | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.addThisAppSource` | Add this app source | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.pickerHint` | Choose an .exe from disk, or type its full path. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.noRunningApps` | No running applications were found. Refresh to try again. | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.runningApplication` | Running application | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.audioActive` | audio active | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.noAudioYet` | no audio session yet; capture begins when it plays sound | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.addRunningApp` | Add running app | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.groupLegend` | Applications in this group | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.createGroup` | Create app group | Application/plugin picker and keyboard connection dialog | none |
| `signalFlowDialogs.afterAdding` | After adding it, select the app's card and open Properties to change how it is captured. | Application/plugin picker and keyboard connection dialog | none |
| `discordRoute.chooseMicrophone` | Choose a microphone. | Discord route refusals | none |
| `discordRoute.chooseCable` | Choose a virtual cable. | Discord route refusals | none |
| `discordRoute.alreadySends` | This session already sends your voice to {{name}}. | Discord route refusals | name |
| `discordRoute.cableTaken` | {{taken}} already plays into {{name}}. Choose another cable. | Discord route refusals | taken, name |
| `svgHandle.hint` | {{label}}: {{value}} {{unit}}. Drag, or use the arrow keys (Shift for bigger steps). | Accessible drag hint on a graph handle | label, value, unit |
| `backendErrors.alreadyPrepared` | Audio is already prepared in this app. Stop playback, then try Play again. If it still cannot start, close AudioRouter and reopen it. | Status line message for a caught backend error | none |
| `backendErrors.mediaNotReady` | An audio file is not ready yet, so the route was not saved. Wait until the file finishes importing, or choose it again, then press Save. | Status line message for a caught backend error | none |
| `backendErrors.settingInvalid` | A tool setting ({{name}}) has a value AudioRouter cannot use, so the route was not saved. Check the tool's Properties, then press Save. | Status line message for a caught backend error | name |
| `backendErrors.revisionConflict` | Another save changed this route, so your save was not applied. Your edits are kept. Compare them with the latest saved route, then save again. | Status line message for a caught backend error | none |
| `backendErrors.deviceInvalidated` | The selected audio device changed or disconnected. Stop audio, refresh the device list, then select the exact input and output again. Play will reopen those devices. If one is missing, reconnect it before retrying. | Status line message for a caught backend error | none |
| `backendErrors.deviceInUseNamed` | "{{device}}" is in use by another application, which has taken it exclusively. Close or reconfigure that application, or stop apps from locking it: Windows Sound settings → {{tab}} → {{device}} → Properties → Advanced → untick "Allow applications to take exclusive control". Then press Play again. | Status line message for a caught backend error | device, tab |
| `backendErrors.deviceInUseNamedWithTwin` | "{{device}}" is in use by another application, which has taken it exclusively. This is a VB-Cable: an application playing to its twin "{{twin}}" (the same cable) also blocks it, so set that application to "{{device}}" instead. Close or reconfigure that application, or stop apps from locking it: Windows Sound settings → {{tab}} → {{device}} → Properties → Advanced → untick "Allow applications to take exclusive control". Then press Play again. | Status line message for a caught backend error | device, twin, tab |
| `backendErrors.deviceInUse` | The selected audio device is in use by another application. Choose a different device in the Properties of the output tool, or release this exact device in the application using it, then prepare it again. | Status line message for a caught backend error | none |
| `backendErrors.unsupportedTopology` | This route includes an audio combination the current engine cannot play. Try a route from a separate Test Signal to the output device, or remove one source and connect the remaining source directly to the output. Your saved route was not changed. | Status line message for a caught backend error | none |
| `backendErrors.permissionDenied` | Permission denied: this app is not granted the {{scope}} permission needed for this action, so the backend refused it. Nothing was changed. | Status line message for a caught backend error | scope |
| `backendErrors.scopeRequired` | required | Status line message for a caught backend error | none |
| `backendErrors.retry` |  Retry may succeed. | Status line message for a caught backend error | none |
| `backendErrors.windowsPlaybackTab` | Playback | Status line message for a caught backend error | none |
| `backendErrors.windowsRecordingTab` | Recording | Status line message for a caught backend error | none |

## Wording changed earlier, still needing a reviewer

These English messages were reworded on 2026-11-24 from engine terms to the
plain glossary ("node" → tool, "draft" → unsaved edits, "physical output" →
output device). Their French and Chinese text now says **outil/工具**,
**modifications non enregistrées/未保存的修改** and **périphérique de
sortie/输出设备** for these keys. Other, older messages still use
`nœud`/`节点`; see the terminology decision below.

`workspaceAudit.notChosen`, `recordingUi.attached`, `workflowUi.startNode`,
`workflowUi.working`, `workflowUi.readyHelp`, `workflowUi.fileMissing_one`,
`workflowUi.fileMissing_other`, `workflowUi.fileCheck`,
`parameterUi.help.mute.muted`, `canvasUi.disabled`, `toolCatalog.audio-file.help`,
`settings.liveAutosave.offHelp`, `g2.nwFooterSend`, `g2.nwFooterRecv`,
`g1.toneNotRouted`, `g5.meterPressPlay`, `g5.appChangeHelp`,
`g4Canvas.recNone`, `h1.detailHears`, `h1.detailPlaysInto`, `h1.spkYourNode`.

## Open wording decisions for the owner or reviewer

1. **`nœud` / `节点` versus tool.** The glossary agrees on "tool" (`outil` /
   `工具`), but older French and Chinese messages still say "node". Decide
   whether to rename every occurrence: `grep -n 'nœud' ui/src/i18n/locales/fr.json`
   and `grep -n '节点' ui/src/i18n/locales/zh-Hans.json`. The keys above are
   already changed; a full rename is a translation edit, not a code edit.
2. **The Play label.** French uses `Lecture` in some sentences and `Démarrer`
   for the button ("Appuyez sur Lecture" versus "cliquez sur Démarrer").
   Choose one term for the Play button and one for playback.
3. **`draftErrors.runningAppOnly` and `scanResultOnly` say "graph".** The
   glossary prefers "route" (`route` in French, `路由` in Chinese). Decide
   whether to reword the English too.
4. **Windows tab names.** `backendErrors.windowsPlaybackTab` and
   `windowsRecordingTab` localize the Sound-settings tab names shown in the
   guidance. If the person's Windows language differs from the app language,
   the path is approximate. Accept, or map the names from the Windows locale
   instead.
5. **The permission scope is a wire value.** `backendErrors.permissionDenied`
   inserts `{{scope}}` as `PluginScan`, `DeviceAdministration`, `GraphWrite`
   and so on. Decide whether to keep the API name (L10N-05 says wire values
   stay) or add a small localized scope-name table beside it.
6. **`bus` in the virtual-cable notices.** French keeps `bus`, as the existing
   catalog did. Confirm the reviewer's preferred term.
7. **Message severity the owner may want changed** (`MESSAGE_TONES` in
   `ui/src/actionMessageTones.ts`, all kept identical to the pre-2026-11-24
   behaviour): `h1.importUnavailable`, `g2.neNoVbCapture`, `g2.neNoVbPair`,
   `g1.appFindingSessionGone`, `g1.appFindingToolGone`, `g1.appOlderEdits`
   (success, green check), and a recording take that stopped early
   (`g1.appRecordingEarly`, success today; a warning may fit `failed`).
8. **French typographic spacing** before `:`, `;`, `!` and `?` and around
   `« »`. The catalog uses the non-breaking-space convention where the text
   was written; confirm it renders as intended at 200 % zoom.

## Not in this queue

Windows-owned dialogs, Narrator pronunciation, the packaged shell and the
tray must be checked on a real Windows session (owner gates). Live audio
switching is a separate owner gate. See the final handback for the full list.