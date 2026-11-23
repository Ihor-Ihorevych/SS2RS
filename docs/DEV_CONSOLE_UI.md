# Developer console and performance overlays (UI side)

`ss2_ui::devconsole` (T10-007) is what is on screen when a developer presses the console key, and the small text blocks that show frame time
and other figures. It sits on the console backend of `ss2_core::console` (T03-015: variables, commands, history, completion) and draws through the
same `DrawList` as every other screen, in canvas units (1920 by 1080).

## `ConsoleView`

The model of the drop-down console. It owns no console: the caller passes the `Console` to `handle_key`, so one view works for the client and
the server.

```rust
let mut view = ConsoleView::new(1000);        // remembers at most 1000 lines
view.toggle();                                // the console key
view.handle_text("echo hi");                  // typed characters (from text input events, not key presses)
view.handle_key(ConsoleKey::Enter, &mut console);   // runs the line; returns false when the console is closed so the game gets the key
let list = view.draw();                       // a DrawList, empty when closed
```

- **Editing:** insert at the cursor, `Left`/`Right`/`Home`/`End`, `Backspace`/`Delete`, by **character** (not byte: "ñ" is one step). Control characters and newlines
  are not typed (a paste is safe); the line is limited to 256 characters.
- **Running:** `Enter` echoes `> line`, runs it and prints the output, or the error in the error colour; an empty line prints nothing.
- **History:** `Up` and `Down` walk the backend's history; past the newest entry is the empty line.
- **Completion:** `Tab` completes the **first word** only: a unique name is completed and followed by a space, several names extend to their common prefix, and if
  nothing can be added the candidates are listed in the scrollback.
- **Scrollback:** bounded (the oldest lines are dropped); `PageUp` and `PageDown` scroll by a page and stop at the ends; **new output snaps back to the bottom**.
- **Drawing:** a dark translucent panel across the top (45 % of the height) with the lines bottom-aligned above the prompt, coloured by kind (what you typed, output,
  errors), and the prompt `> abc|def` with the cursor shown as a bar. Only the lines that fit are drawn.
- **Closed:** `Escape` closes it; a closed console uses no key and no text and draws nothing.

## `FrameStats` and the overlay

`FrameStats::new(window)` keeps the latest frame times: `fps()` (from the average), `average_ms()`, `max_ms()` and **`one_percent_low_fps()`** (the frame
rate of the slowest 1 % of the frames in the window, at least one frame: what a stutter feels like when the average hides it). A frame counts as at most a
minute, so a stall cannot make the numbers meaningless. `stats_lines(&stats)` gives the text lines ("60 fps", "16.7 ms avg, 16.7 ms max", "1% low: 60 fps"),
and `draw_overlay(&lines, Corner::TopRight)` puts any lines of text (at most 32) on a translucent panel in a corner. The same overlay carries the
network figures once something feeds it (`ss2_net::netstats::NetGraph`, T08-018b).

## How it was tested

Twenty unit tests, written first and seen to fail on the missing API: opening and closing; typing and moving the cursor; backspace and delete; editing by character with
two-byte characters; control characters and the length limit; running lines and printing output and errors; empty lines; history; completion (unique, common prefix,
candidates listed, only the first word, unknown prefix); the bounded scrollback; paging with the ends and the snap back to the bottom; the drawn panel and prompt, the colours,
fitting inside the panel and drawing only what fits; frame statistics (steady rate, the window, a hitch in the maximum and the 1 % low, zero and huge frame times); the
overlay in each corner, bounded and empty. Fifteen deliberate breakages each fail a test (a closed console consuming keys, Escape doing nothing, the cursor running past
the end, control characters typed, an unbounded line, errors not marked, no space after a completion, an unbounded scrollback, new output not snapping to the bottom,
scrolling past the oldest line, a backspace by byte, a 1 % low of one frame, no frame time cap, an unbounded overlay). **Three survived at first** (the byte-versus-character
backspace, the 1 % low with a longer window, and the frame cap: my tests used cases where both versions agree) and I added the cases and re-ran them to see them fail.
One more is **equivalent**: the check that stops completion of arguments is redundant, because a line with a space in it matches no name anyway.

## Not covered (follow-up T10-007b)

- **Nothing calls it yet.** The app has to map its key events onto `ConsoleKey` and its text events onto `handle_text`, toggle on the console key, draw `view.draw()` last,
  and feed `FrameStats` the frame times; that is the app's input wiring (T03-033). I have **not seen it on a screen**: the draw list is checked by tests, not by a golden image.
- **Text metrics.** The cursor is a bar character inside the text (the UI has no font metrics to place a caret exactly), so with a proportional font the line may shift slightly as the bar moves.
- Selection, copy and paste, a mouse wheel, a **key to scroll to the end**, a log line filter, and **console variables** for the overlays (`r_show_fps`, `net_graph`).
- The **network overlay** itself (T08-018b) and **GPU timing** (T04-019).
