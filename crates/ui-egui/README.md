# FilmCraft interface

The egui frontend draws panels and dialogs, and dispatches project changes through the engine.
Interactive controls keep stable automation ids regardless of the interface language.

## Waveform peak pyramid (#106)

Timeline clips and the Source monitor share an immutable `PeakPyramid` cached per media item
and audio stream. Nested-sequence caches retain their existing content-key invalidation.
Decoding/mixing appends the original zero-inclusive min/max pair for each 256-sample bucket
at 48 kHz, including the final partial bucket, and builds completed binary ancestors immediately.
Building is O(N) overall, amortized O(1) per append; no PCM copy is retained. Source amplitude
is accumulated during building rather than scanned on the first paint.

A column keeps its previous time-to-bucket rounding and half-open range. Small ranges scan
at most eight base buckets; larger ranges use aligned power-of-two blocks, at most two per
level, for exact O(log N) queries. Boundary buckets are never broadened to a coarser level.
This preserves trimming, scroll offsets, speed/reverse/hold mapping, gain, and linear/log display.

For N base buckets the pyramid stores `2*N - N.count_ones()` pairs of two `f32`s. One hour
of 48 kHz audio is 675,000 buckets: 10,799,928 bytes of pair payload (10.30 MiB), plus about
480 bytes of level descriptors. The two-hour benchmark uses 21,599,928 bytes of pairs.
This is just under twice the flat-cache payload; stereo does not double it because FilmCraft
already displays the first channel of each audio stream. Separately cached streams each pay
this cost. While appending, vector capacities are bounded by `4*N + 4*levels` pairs; publishing
calls `finish()` to release spare capacity. Tiny inputs have the usual fixed Vec overhead.

The existing benchmark runner includes a `waveform` section, with synthetic peaks for a
two-hour 48 kHz stereo clip, 1,000 columns, five zooms, and nonzero scroll offsets. It times
CPU repaint work (egui pass, column queries, mesh construction/submission), not decoding,
full editor layout, tessellation, or GPU rendering. Each case warms up ten frames and times
200 frames. Pyramid construction is outside the repaint measurement and reported separately.
Use `--only flat` to reproduce the old scan, or omit it to measure the pyramid:

```sh
export CARGO_TARGET_DIR="$PWD/target-106" CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0
cargo +stable run --config 'profile.dev.package.filmcraft-ui-egui.opt-level=3' \
  -p filmcraft-ui-egui --example bench -- --section waveform --cpu --only flat --repeat 3
cargo +stable run --config 'profile.dev.package.filmcraft-ui-egui.opt-level=3' \
  -p filmcraft-ui-egui --example bench -- --section waveform --cpu --repeat 3
```

Tests compare all ranges after every append, randomized ranges, and every drawn column of
randomized timeline/monitor zooms and scroll offsets against the original flat scan. A real
timeline-painter test compares its exact mesh with the flat-scan mesh at five zooms in both
linear and dynamic modes. Existing stream-selection, nested-sequence refresh, and project-reset
tests continue to exercise cache behavior.

Measured on November 3, 2026, Apple M6/arm64, rustup stable rustc 1.99.0, with the command
above (UI crate opt-level 3, dependency opt-level 2, debug information/incremental disabled).
The original pre-change flat baseline was 0.2243/0.0513/0.0217/0.0087/0.0112 ms at the five
zooms below. The matched final runs rerun that same flat scan with `--only flat`; values are
the median of five runs' frame p50/p95 timings, under shared-machine load (15.92/24.50/24.30).

| Visible span | Flat p50 ms | Pyramid p50 ms | Flat p95 ms | Pyramid p95 ms |
| --- | ---: | ---: | ---: | ---: |
| 2 hours | 0.234709 | 0.025500 | 0.384500 | 0.030875 |
| 30 minutes | 0.072333 | 0.024500 | 0.084375 | 0.033250 |
| 5 minutes | 0.022542 | 0.019500 | 0.044667 | 0.026458 |
| 30 seconds | 0.012709 | 0.012458 | 0.014250 | 0.014500 |
| 1 second | 0.011292 | 0.011542 | 0.012250 | 0.012834 |

Building the two-hour pyramid from decoded buckets took 6.47 ms once (PCM decoding excluded).
Whole-clip repaint p50 improves 9.2×; close zooms remain approximately unchanged.
Validation: `cargo +stable test -p filmcraft-ui-egui` passed 1,077 tests, zero failures, six
pre-existing ignored; scoped `cargo +stable clippy -p filmcraft-ui-egui --all-targets -- -D warnings`,
formatting, layers and assets checks passed. Full-workspace tests/clippy and the workspace wasm
gate were skipped because the shared disk fell below 7 GiB free; `target-106` was removed.

## Export range warnings

The Export panel shows the selected range's length in sequence frames and seconds beside Range
(`export.range.duration`). When Source In/Out covers just one sequence frame and at least one
mark is set, `export.range.warning` explains how to export the whole sequence instead (#3364).
The warning is advisory: exporting one frame remains valid. Entire Source, Work Area and Custom
ranges do not show this warning. `export_ui::single_frame_in_out_warning_updates_with_range_and_marks`
checks the warning and duration as marks and range selections change; the same test can render
`export-single-frame-warning.png` with `FILMCRAFT_UI_SNAPSHOT_DIR`.

## Control input limits

The control channel bounds `ui.click` to 10 clicks and `ui.drag` to 1024 movement steps (#2784).
The MCP bridge sends null for omitted counts/steps, so the validator treats null like an absent
field: one click or 12 drag steps. Negative, fractional, non-numeric and over-limit values still
return errors before queuing any input. Zero clicks emit no button events; 0–1 drag steps use 2.
`control::input_bounds_tests` covers defaults, rejection, unchanged pending input and valid
gestures; `keyboard_ui` verifies that rejected requests leave the UI usable. See the
[control protocol](../../docs/control-protocol.md).

## Transition drop previews

Dragging a video or audio transition over a compatible clip shows its actual time span, the
selected cut and an In/Out label. Shared cuts center the span; isolated edges use a one-sided
span. The preview uses the engine's read-only transition planner, including Timeline duration
preferences, and release dispatches `effects.apply` with the same edge. Locked tracks and
incompatible clip types show no valid transition preview. The drop snaps to Start at Cut, Center
at Cut or End at Cut by where the pointer is relative to the cut. Ordinary effects retain their
clip outline. `timeline.transitionDropPreview` exposes the span and alignment to automation.

`transition_drop_ui` covers the three alignments on video/audio cuts and a one-sided clip edge,
hover without project edits, the committed span and undo. Set `FILMCRAFT_UI_SNAPSHOT_DIR` to
render screenshots with wgpu.

## Localisation

Edit > Language offers English, Japanese, Spanish, Brazilian Portuguese, Ukrainian, Simplified
Chinese, German, Russian, French, Indonesian, Korean, Italian, Czech, Polish, Swedish and Turkish (all but Ukrainian cover
the whole interface; Ukrainian covers the core menus). The language is stored in the engine's `general.interfaceLanguage` preference,
restored on startup, and also available in Settings > General.
Its default, System Language (`system`), follows the operating system: the first of the user's
preferred languages that the interface has (the host supplies them through
`HostHooks::system_languages`: `sys-locale` on the desktop, `navigator.languages` on the web),
otherwise English.
The `app.language.*` UI commands and `prefs.set` reach it through the control channel.
Japanese requires a craft-fonts build or a suitable installed font.

English source strings are lookup keys in `src/i18n/<code>.tsv`. `tl!` translates literals, `tlf!`
fills translated templates, and `i18n::t` translates names from registries. Placeholder values
(including user filenames containing braces) are inserted literally. Catalog translations are
original work using ordinary language, without proprietary localisation resources.

Spanish, Japanese, Simplified Chinese, French, Brazilian Portuguese, German and Russian cover
menus, panels, dialogs, settings and registry labels. Searches accept both the
translated label and its English source, including Unicode capitals. Project content, command ids
and preference values retain their original values. Engine errors, CLI and MCP messages remain
English. Ukrainian currently covers the core menus; it falls back to English elsewhere.

A string a catalog lacks shows in English, so a missing translation never fails a test or a
build. `cargo xtask i18n-report` prints each language's missing strings (menus, `tl!` literals and
registry names) and its entries whose source the interface no longer has; `cargo xtask ci` prints
the same report in its log.

Verification: `cargo test -p filmcraft-ui-egui` checks catalog syntax, duplicate keys, placeholders
and UI behaviour in other languages (switching, search, font coverage); `cargo xtask ci` runs the
workspace gates. Visual checks use the control channel to switch language and capture the resulting
panels.

## Source playback

`source_playback.rs` auditions media independently of Program playback. It reuses MediaSource
decoding, output mapping, the device clock and the desktop play-ahead buffer. Only one monitor
owns the device at a time. Source frame jobs use the existing frame server and Source resolution.
The live project, undo stack and timeline playhead are unchanged. See [monitor documentation](../../docs/monitors.md)
for behavior, commands, limitations and regression tests. Source playback tests use synthetic
demo media and fake audio outputs; no third-party assets are introduced.

Source drag controls snapshot the selected span and dispatch `timeline.place` for video, audio,
or linked video/audio. Full-clip marks are implicit until set by the user. The engine owns edits,
range validation and undo; the UI owns the drag gesture. See `source_drag_ui` and the engine's
`source_placement_tests` for regression coverage.

Source range handles use transient gesture previews and the engine's integer-frame
`source_monitor::adjust_range`; release dispatches one mark edit. Monitor command routing
keeps Source navigation, markers and marked-range playback separate from Program.

## Pane layout controls

The panels use `craft-ui::docking` with its split-tree settings: craft-ui draws the frames, tab
strips, floating windows and drag previews and owns the gestures; `dock_ui.rs` draws the panel
bodies, the ≡ panel menu and the Timeline's sequence tabs. Bounded split geometry and
keyboard-accessible resize handles use `craft-ui::layout`. Panel tabs or their group's drag bar
move between tab groups, split at a group's or the window's edge, reorder, or float when released
with Cmd/Ctrl; a release over no target cancels. Tools and the Audio Meters keep their widths
as the window resizes (fixed split sizes, which a divider drag keeps fixed); the other groups
resize with the window. Every group resizes down to the same minimum and has no maximum.
Timeline sequence tabs retain their sequence-opening, closing and horizontal reorder behavior.

On desktop each floating panel opens in its own OS window. Tool panels are palettes: they stay
above other windows and hide while another application is active. The video viewers (Program,
Source, Reference Monitor, Lumetri Scopes) are normal windows, say on a second display: they
stack with the main window and stay shown in the background. A floating window moves by its
title bar, resizes from every edge and splits like the main window. While a modal dialog is open
(Settings, Recovery, Revert, a file dialog), the native windows are dimmed and take no input.
The main window's input rewrite (macOS Control+Click as a secondary click,
`--control-input-only`) also runs in each native window, and files dropped on one are imported
like files dropped on the main window; on the Timeline they land at the drop point. A drag from
a panel (a clip from the Source Monitor or the Project panel, an effect, a template) drops on a
panel in any of the app's windows. On Wayland
floating windows stay inside the main window. Where egui embeds viewports (tests), floating
windows stay inside the main window too.

Named workspaces save the layout, floating windows included; workspaces saved in the earlier
layout format still load. While every floating window is one tab group, a save also writes the
earlier format, so earlier versions still open the workspace. Automation IDs and workspace commands remain the integration boundary.

Tools and dock tabs expose their actual egui widget IDs, paint keyboard focus, and support
Tab/Shift-Tab traversal and Space/Enter activation. Control-injected keys can reach focused
widgets in a background window. The optional `--control-input-only` native launch mode isolates
automation from native widget/menu input; `ui.inspect` reports native and effective focus
separately. Focus and input-ownership regressions cover document/playhead preservation.

Compact monitor layouts separate the current and duration timecodes from zoom, resolution and
settings. Source video/audio drag choices retain their existing placement behavior. Transport
buttons use `craft-ui::layout::fit_toolbar`; commands beyond the available width remain available
through More, including their selected state and Source/Program routing. A scrolling fallback
retains all commands if even More cannot fit. `monitor_geometry` checks the painted readouts,
control bounds, overflow actions and Source drag payloads across monitor widths and themes.

## Marker properties

Right-click a timeline or Markers-panel marker and choose **Edit Marker…**. The window
edits time/duration, name, notes, author, type, and color. Existing presets are
shown as swatches; project custom swatches are below them. Done makes one undoable marker
edit; Cancel discards the marker draft.
The compact form orders Name, Author, Time/Duration, scrolling Notes, then Color and
marker types. Previous/Next save valid changes before opening the neighbouring marker;
unchanged navigation adds no Undo step. Unchecked Duration saves a point marker even
when its disabled field retains a draft value. Timecodes require complete numeric fields
with colons; Done stays disabled until timing is valid. Text fields support normal
clipboard shortcuts and local text Undo/Redo.

The **+** square below the presets opens a separate color editor. macOS uses the native
Colors panel; other hosts and native-picker failures reuse the established egui picker.
FilmCraft retains exact six-digit sRGB hex entry and explicit confirmation. Add color
saves a reusable swatch to this project's palette through `colors.addCustom` (a separate
undoable operation) and selects it in the marker draft. Canceling the marker edit afterward
does not remove a deliberately saved palette color. The project stores at most 64 custom
swatches shared by markers and clips; duplicate additions are no-ops. Used custom colors also appear for reuse.
Marker/color dialogs belong to the opened project instance, close when that project is replaced,
and are not restored from saved UI preferences. Unrelated edits keep marker drafts open;
editing or removing that marker elsewhere closes its stale draft rather than overwriting newer data.

Markers and clips now share the existing 16 clip preset colors; clip RGB values and preset
names are unchanged. Markers no longer collapse those names onto seven display colors.
Red/Rose, Orange/Mango, Cyan/Teal and Pink/Magenta resolve to the same presets. Exact custom colors
(including white, black, pink, and gray) override a marker's preset. Preset color filters
leave custom markers visible. Selecting a preset clears the override, preserving the author. The preset colors also follow Settings → Labels in both views.

The opaque sRGB editor lives in `widgets::color_picker`, independently of marker models,
dialogs and project commands. Callers provide a `ColorDraft`, automation registry and id
prefix. Its HSV conversion operates on sRGB channels; using linear-light saturation would
make the center too white. Lumetri's grading wheels remain separate: they edit grading
offsets. Existing effect/graphics pickers retain their alpha/float/color-space semantics.

The same author and hex values are available to Source marker commands. Existing markers
have an empty author; FilmCraft never infers authorship from the operating-system account.
Projects (schema 17) retain marker properties, Web Link URLs and the project palette. Marker CSV reports
and FilmCraft OTIO metadata retain individual marker authors, URLs and colors. OTIO also
retains custom clip label colors and the palette in FilmCraft metadata. Other interchange exports warn when they cannot preserve those marker properties.
Importing an OTIO document merges its saved swatches after the current project's swatches, omitting duplicates and respecting the 64-swatch limit.
This does not add AVB marker import or Avid track-bound markers.

Regression coverage includes project palette persistence/isolation/bounds/undo, marker
commands and reopen, separate picker and marker drafts, preset/custom swatch reuse,
sRGB wheel conversion, CSV escaping, and OTIO round trips.

Clip and Project-panel Label menus expose the same project custom palette below the presets.
Add custom color opens the shared picker; Add and apply saves the swatch and applies it to
the captured selection in one undoable operation. `edit.label` accepts a preset/alias or hex,
explicit `clips`/`items` targets, and optional `saveColor:true`. Choosing a named preset clears
that item's custom override. Duplicated items and newly inserted source clips retain the exact
custom label color. Bright custom timeline fills use dark text for readability.

The marker editor also edits exact frame time and duration, switches between point and range
markers, and deletes by marker ID. Double-click a timeline, monitor, or panel marker to edit it;
right-click exposes Edit Marker and Delete Marker. Source and sequence ownership is shown and
captured at opening, so changing the open clip or sequence does not redirect edits. Unchanged
time fields preserve imported subframe ticks. A source range cannot extend beyond its source.

The Markers panel switches between Sequence and Source, searches name/notes/author, filters
preset and custom colors, and supports Command/Ctrl-click, Shift-click, and Select all for the
visible rows. Bulk color changes and deletion are each one undo step. Hidden rows leave the
selection. `markers.edit` and `markers.delete` take an explicit `sequence` (program) or `item`
(source) and either `marker` or `markers`; invalid IDs reject the entire operation.

Manage Colors is available from the Markers panel and clip Label menus. The separate project
palette manager adds, names, edits, and removes swatches using the shared color picker. Editing
or removing a saved swatch preserves already assigned hex values; used colors remain available
for reuse. `colors.addCustom`, `colors.updateCustom`, and `colors.removeCustom` are undoable.
Names persist in the project and FilmCraft OTIO metadata; existing sixteen presets are unchanged.

### Editable clip duration

The Clip Speed / Duration dialog lets you type a duration as timecode. Duration and
Speed describe the same source span: editing either updates the other. Duration uses
the sequence frame rate; invalid, zero, or out-of-range durations cannot be applied.
The field is available to automation as `speedDuration.duration`. OK uses the existing
speed command, including its linked-clip, ripple, neighbour, and Undo behaviour;
Cancel discards the draft. The selected clip's duration is the reference when more
than one clip is selected, matching the existing Speed control.

`clip_dialogs_ui::duration_text_edits_speed_and_rejects_invalid_input` exercises real
text input, validation, application, Cancel, and Undo/Redo.
