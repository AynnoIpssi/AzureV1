# Azure Engine — Technical Documentation

**Version:** V1.5
**Language:** Rust (2024 edition)
**Platform:** Linux (Wayland)
**Status:** Active development — rendering module operational, variable-font text rendering operational

---

## Table of Contents

1. [What is Azure Engine?](#1-what-is-azure-engine)
2. [Where it fits in the Azure ecosystem](#2-where-it-fits-in-the-azure-ecosystem)
3. [Core concepts you need to understand first](#3-core-concepts-you-need-to-understand-first)
4. [Architecture overview](#4-architecture-overview)
5. [Project structure](#5-project-structure)
6. [Models — data structures](#6-models--data-structures)
7. [Managers — logic and operations](#7-managers--logic-and-operations)
8. [Rendering module](#8-rendering-module)
9. [The full window creation sequence](#9-the-full-window-creation-sequence)
10. [Dependencies](#10-dependencies)
11. [Variable font weight support](#11-variable-font-weight-support)
12. [Text layout correctness — baseline, side bearings, minimum spacing](#12-text-layout-correctness--baseline-side-bearings-minimum-spacing)
13. [Stem darkening](#13-stem-darkening)
14. [Rendering correctness — bugs found and fixed](#14-rendering-correctness--bugs-found-and-fixed)
15. [Hinting — what it is, and why we're not implementing it](#15-hinting--what-it-is-and-why-were-not-implementing-it)
16. [Test suite for text rendering](#16-test-suite-for-text-rendering)
17. [Current capabilities and limitations](#17-current-capabilities-and-limitations)
18. [What comes next](#18-what-comes-next)

---

## 1. What is Azure Engine?

Azure Engine is the lowest-level runtime component of the Azure platform. Its single responsibility is to bridge the gap between Azure applications and the operating system's display infrastructure — in practical terms, it answers the question: *how does a Rust program get a visible, persistent, interactive window on screen, without using any UI framework?*

Every graphical application needs to go through a series of system-level steps before it can render anything: it must establish a communication channel with the display server, negotiate a memory region that both the app and the display server can access, declare the existence of a surface, confirm a configuration handshake, and then continuously process events to stay alive and responsive. Azure Engine handles all of these steps from scratch, speaking the raw binary Wayland protocol directly over a Unix socket.

This is intentionally the hardest possible approach. No bindings library, no abstraction layer, no framework. Every byte sent to the compositor is constructed by hand, in the exact format defined by the Wayland protocol specification.

---

## 2. Where it fits in the Azure ecosystem

Azure is organized in five layers, from the most abstract to the most concrete:

```
┌─────────────────────────────────────┐
│           Applications              │
│   AzureWork · AzureMail · AzureDev  │
├─────────────────────────────────────┤
│         Azure Foundation            │
│  Shared UI · Navigation · Manifests │
├─────────────────────────────────────┤
│           Azure Services            │
│  Identity · Permissions · Search    │
├─────────────────────────────────────┤
│             Azure Core              │
│  Contracts · Events · Registry      │
├─────────────────────────────────────┤
│             Azure Engine            │
│  Wayland · Window · Input · Render  │
└─────────────────────────────────────┘
```

Azure Engine sits at the very bottom of the stack. Azure Core defines the logical rules of the platform. Azure Engine makes it visible. Foundation and all applications depend only on the `AzureWindowProvider` trait defined in Core — never on Wayland-specific code directly.

---

## 3. Core concepts you need to understand first

Before reading the code, three concepts are essential.

**The compositor** is the program responsible for managing all windows on screen. On a Linux desktop running GNOME, this is Mutter. It is an ordinary user-space process, but it holds a privilege that other programs do not: direct access to the screen hardware. Every application that wants to display something must go through the compositor. The compositor receives each app's content, composites everything together into one final image, and sends that image to the physical screen.

**Wayland** is the protocol that applications use to communicate with the compositor. It is a binary protocol exchanged over a Unix socket — a special file on disk (`/run/user/1000/wayland-0`) that acts as a bidirectional pipe between two processes. Every message has a fixed structure: a 4-byte object ID (who is this message addressed to), a 4-byte combined field (message size in the upper 16 bits, opcode in the lower 16 bits), and any number of 4-byte arguments. All integers are encoded in little-endian byte order.

**Shared memory** is the mechanism that lets an application's pixel data reach the compositor without being copied. The application allocates a region of memory using the `memfd_create` system call, fills it with pixel data, and then transmits the file descriptor (a number referencing that memory region) to the compositor using a special kernel mechanism called `SCM_RIGHTS`. Once the compositor receives this file descriptor, both processes point to the exact same physical RAM — there is no network transfer, no data copy, just two processes reading and writing the same bytes.

**The pixel buffer** is a flat 1D array of bytes in shared memory. Every pixel occupies exactly 4 bytes in BGRA order (Blue, Green, Red, Alpha) — this matches the `ARGB8888` format declared when the `wl_buffer` is created (`wl_shm.format` value `0`), which the Wayland spec defines as a 32-bit `0xAARRGGBB` value stored little-endian, i.e. bytes `[B, G, R, A]` in memory. The position of pixel `(x, y)` in this flat array is computed as `(y * width + x) * 4`. Wayland requires the `damage_buffer` call before each `commit` to signal which region of the buffer has changed.

> **Every pixel-writing function in the rendering module writes directly in this B, G, R, A byte order.** There is no separate "internal RGBA" representation anywhere — see [§14.6](#146-red-and-blue-channels-swapped-everywhere) for the history of why this matters and how it was verified.

---

## 4. Architecture overview

The codebase follows a strict model/manager/service separation:

**Models** hold data. They are plain structs with constructors and getters, no business logic.

**Managers** hold logic. They are collections of free functions that operate on models and send/receive messages over the connection.

**Services** (in the rendering module) are isolated operations on the canvas — each shape, effect, or utility is a separate file.

The `Window` struct is the central object exposed to the rest of the platform. It owns the `WaylandConnection`, all Wayland object ids, the pixel pointer, and the dimensions. It implements the `AzureWindowProvider` trait from Azure Core, making it usable by Foundation and applications without any Wayland-specific knowledge on their side.

---

## 5. Project structure

```
azure-engine/
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── Sora-VariableFont_wght.ttf
    ├── Roboto-VariableFont_wdth,wght.ttf
    ├── platform/
    │   ├── mod.rs
    │   └── wayland/
    │       ├── mod.rs
    │       ├── models/
    │       │   ├── mod.rs
    │       │   ├── connection.rs
    │       │   ├── registry.rs
    │       │   ├── shared_memory.rs
    │       │   ├── window.rs
    │       │   ├── screen_output.rs
    │       │   └── object_id_allocator.rs
    │       └── managers/
    │           ├── mod.rs
    │           ├── connection_manager.rs
    │           ├── registry_manager.rs
    │           ├── bind_manager.rs
    │           ├── shared_memory_manager.rs
    │           ├── shm_manager.rs
    │           ├── compositor_manager.rs
    │           ├── xdg_manager.rs
    │           ├── surface_manager.rs
    │           ├── output_manager.rs
    │           └── window_manager.rs
    └── rendering/
        ├── mod.rs
        ├── models/
        │   ├── mod.rs
        │   ├── color.rs
        │   ├── pixel.rs
        │   ├── canvas.rs
        │   └── glyph.rs
        ├── services/
        │   ├── mod.rs
        │   ├── buffer.rs
        │   ├── effects.rs
        │   ├── shapes/
        │   │   ├── mod.rs
        │   │   ├── rect.rs
        │   │   ├── line.rs
        │   │   └── circle.rs
        │   └── text/
        │       ├── mod.rs
        │       ├── loader.rs
        │       ├── glyph.rs
        │       ├── renderer.rs
        │       └── kerning.rs
        └── managers/
            ├── mod.rs
            └── renderer.rs
```

Two font files now ship with the engine: **Sora** (`wght` axis 100–800, default 400) and **Roboto** (`wght` axis 100–900, `wdth` axis 75–100, default 100/400) — both variable fonts, used throughout the test suite to validate the text pipeline across different families and axis ranges.

---

## 6. Models — data structures

### `models/connection.rs` — `WaylandConnection`

The low-level socket wrapper. Wraps a `UnixStream` and exposes four communication methods.

**Fields:**
- `stream: UnixStream` — the open socket connected to the Wayland compositor.

**Methods:**

`new(stream: UnixStream) -> WaylandConnection`
Constructs a connection from an already-open stream.

`send(&mut self, data: &[u8]) -> Result<(), String>`
Sends a slice of bytes over the socket.

`send_with_fd(&mut self, data: &[u8], fd: RawFd) -> Result<(), String>`
Sends bytes alongside a file descriptor using `sendmsg` with `SCM_RIGHTS`.

`receive(&mut self, buf: &mut [u8]) -> Result<(), String>`
Reads exactly `buf.len()` bytes from the socket.

`set_nonblocking(&self, nonblocking: bool) -> Result<(), String>`
Switches the socket between blocking and non-blocking mode.

---

### `models/registry.rs` — `WaylandGlobal` and `WaylandRegistry`

**`WaylandGlobal`** — a single service announced by the compositor.
Fields: `name: String`, `version: u32`, `id: u32`.

**`WaylandRegistry`** — the complete list of services the compositor offers.
Fields: `globals: Vec<WaylandGlobal>`.

---

### `models/shared_memory.rs` — `WaylandMemory`

Fields: `fd: i32`, `size: usize`.

---

### `models/window.rs` — `Window`

The central platform object. Implements `AzureWindowProvider` from Azure Core.

**Fields:** `surface_id`, `buffer_id`, `pool_id`, `xdg_toplevel_id`, `xdg_wm_id`, `xdg_surface_id`, `width: i32`, `height: i32`, `ptr: *mut u8`, `connection: WaylandConnection`.

**`AzureWindowProvider` implementation:**
- `render(pixels: &[u8])` — copies pixel buffer into shared memory.
- `poll_event() -> Option<WindowEvent>` — non-blocking event polling.

---

### `models/screen_output.rs` — `ScreenOutput`

Holds the physical resolution of a `wl_output` (monitor) as reported by the compositor.

```rust
pub struct ScreenOutput { pub width: i32, pub height: i32 }
```
`ScreenOutput::new(width, height) -> ScreenOutput`

Populated by `output_manager::get_screen_resolution()` — see [§7](#managers-output_managerrs).

---

### `models/object_id_allocator.rs` — `ObjectIdAllocator`

Monotonic counter for Wayland object ids starting at `4`.

---

### `rendering/models/color.rs` — `Color`

```rust
pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
```
`Color::new(r, g, b, a) -> Color`

Colors are always expressed in normal R, G, B, A order at the API level — callers never think about Wayland's byte order. The B/G/R reordering happens only at the three places that actually write into `canvas.buffer` (`set_pixel`, `blend_pixel`, `draw_glyph`'s blend step). See [§14.6](#146-red-and-blue-channels-swapped-everywhere).

---

### `rendering/models/pixel.rs` — `Pixel`

```rust
pub struct Pixel { pub x: u32, pub y: u32, pub color: Color }
```

---

### `rendering/models/canvas.rs` — `Canvas`

```rust
pub struct Canvas { pub width: u32, pub height: u32, pub buffer: Vec<u8> }
```
`Canvas::new(width, height) -> Canvas` — allocates `width * height * 4` bytes initialized to zero.

---

### `rendering/models/glyph.rs` — `Glyph`

Holds a rasterized glyph's geometry after extraction from a TTF font.

```rust
pub struct Glyph {
    pub width: f32,
    pub height: f32,
    pub advance_width: f32,
    pub descent: f32,
    pub lsb: f32,
    pub contours: Vec<Vec<(f32, f32)>>,
}
```

- `width` / `height` — ink bounding box in scaled pixels (`(bbox.x_max - bbox.x_min) * scale`, same for height).
- `advance_width` — horizontal cursor advance after drawing this glyph (from the font's `hmtx`/`HVAR` data, already reflecting the current variable-font weight).
- `descent` — how far the glyph's ink dips **below** the baseline (`y = 0` in font units). Zero for letters that sit on the baseline (`H`, `e`, `l`...), positive for descenders (`g`, `p`, `q`, `j`, `y`). Added so `draw_text` can align every glyph on one shared baseline instead of aligning their bounding boxes, which used to pull descenders up to the same line as everything else. See [§14.3](#143-descenders-not-hanging-below-the-baseline).
- `lsb` — left side bearing: the gap between the pen position and where the ink actually starts. Added because the contours are normalized to start at local `x = 0`, which silently threw away each glyph's natural left margin and made spacing look uneven. See [§14.4](#144-left-side-bearing-collapsed-to-zero).
- `contours` — the outline as a list of closed polygons, each a list of `(x, y)` points in `[0, width] × [0, height]` space (origin at bottom-left, matching font coordinate conventions). **Contours from the same glyph can legitimately overlap** (e.g. the crossbar and stem of a `t`) — this is why the rasterizer uses a nonzero-winding fill rather than even-odd; see [§14.1](#141-holes-at-stroke-junctions-even-odd-vs-nonzero-winding).

---

## 7. Managers — logic and operations

### `managers/connection_manager.rs`
`connect() -> Result<WaylandConnection, String>` — opens the Wayland socket.

### `managers/registry_manager.rs`
- `get_registry(connection) -> Result<WaylandRegistry, String>`
- `find_global(registry, interface) -> Option<u32>`

### `managers/bind_manager.rs`
`bind_global(connection, name, interface, version, new_id) -> Result<u32, String>`

### `managers/shared_memory_manager.rs`
- `create_shared_memory(size) -> Result<WaylandMemory, String>`
- `map_memory(memory) -> Result<*mut u8, String>`
- `unmap_memory(ptr, size) -> Result<(), String>`

### `managers/shm_manager.rs`
- `create_shm_pool(connection, fd, size) -> Result<u32, String>`
- `create_buffer(connection, pool_id, width, height) -> Result<u32, String>` — declares the `wl_buffer` with format `ARGB8888` (code `0`) and stride `width * 4`. This format choice is what dictates the B,G,R,A memory layout described in [§3](#3-core-concepts-you-need-to-understand-first) — `ARGB8888`/`XRGB8888` are the only two formats every Wayland compositor is required to support, so the engine targets `ARGB8888` rather than a format that would match a more "natural" R,G,B,A layout.

### `managers/compositor_manager.rs`
`create_surface(connection, compositor_id) -> Result<u32, String>`

### `managers/xdg_manager.rs`
- `get_xdg_surface(connection, xdg_wm_base_id, surface_id) -> Result<u32, String>`
- `get_toplevel(connection, xdg_surface_id) -> Result<u32, String>`
- `ack_configure(connection, xdg_surface_id, serial) -> Result<(), String>`
- `attach(connection, surface_id, buffer_id) -> Result<(), String>`
- `set_title(connection, toplevel_id, title: &str) -> Result<(), String>` — sends `xdg_toplevel.set_title`. Encodes the title as a length-prefixed, null-terminated, 4-byte-padded string per the Wayland wire format.

### `managers/surface_manager.rs`
- `commit(connection, surface_id) -> Result<(), String>`
- `damage_buffer(connection, surface_id, x, y, width, height) -> Result<(), String>` — signals to Wayland which region has changed. Must be called before every `commit` after drawing.
- `wait_for_configure(connection, xdg_surface_id) -> Result<u32, String>`
- `run_event_loop(window, xdg_surface_id, xdg_toplevel_id, surface_id, xdg_wm_base_id) -> Result<(), String>`

### `managers/output_manager.rs`
`get_screen_resolution() -> Result<ScreenOutput, String>`
Opens its own short-lived connection, binds `wl_output`, and listens for the `geometry`/`mode` event (opcode `1`) to read the monitor's physical `width`/`height`, returned as a `ScreenOutput`.

### `managers/window_manager.rs`
`window_create(width, height) -> Result<Window, String>` — full creation sequence.

---

## 8. Rendering module

The rendering module is a CPU-based software renderer operating on a flat pixel buffer. It is entirely independent of Wayland — it writes to a `Canvas` in memory, and the caller copies the canvas to the window's shared memory before committing.

### The rendering pipeline

```
draw_*(canvas)           → writes pixels into canvas.buffer
copy_nonoverlapping()    → copies canvas.buffer into window.ptr()
damage_buffer()          → marks the region as changed
attach() + commit()      → Wayland displays the result
```

### Pixel addressing

Every pixel is addressed by the formula `(y * width + x) * 4`. Every function that writes into `canvas.buffer` writes those 4 bytes directly in **B, G, R, A** order — the order Wayland's `ARGB8888` buffer format expects in memory (see [§3](#3-core-concepts-you-need-to-understand-first) and [§14.6](#146-red-and-blue-channels-swapped-everywhere)). There is no intermediate RGBA representation anywhere in the canvas; callers just pass normal `Color { r, g, b, a }` values and the reordering happens inline at each write site.

---

### `rendering/services/buffer.rs`

**`get_pixel_index(x, y, width) -> usize`**
Returns the byte offset of pixel `(x, y)` in the flat buffer.

**`set_pixel(buffer, x, y, width, height, color)`**
Writes `[color.b, color.g, color.r, color.a]` at the correct offset. Bounds-checked — silently ignores out-of-bounds writes.

---

### `rendering/services/shapes/`

**`rect.rs`**

`draw_rect(x, y, width, height, color, canvas)`
Fills a solid rectangle by iterating over all pixels in the bounding box.

`draw_rect_rounded(x, y, width, height, radius, color, canvas)`
Rectangle with rounded corners. Composed of three filled rects and four `draw_circle_filled` calls at the corners.

---

**`line.rs`**

`draw_line_horizontal(x, x_end, y, color, canvas)`
Horizontal line from `x` to `x_end` at fixed `y`.

`draw_line_vertical(y, y_end, x, color, canvas)`
Vertical line from `y` to `y_end` at fixed `x`.

`draw_line(x, y, x_end, y_end, color, canvas)`
General line via a signed-distance-field approach with `apply_aa`, blended with `blend_pixel`. Handles all angles and directions. Uses `i32` coordinates to support all four directional combinations.

---

**`circle.rs`**

`draw_circle(cx, cy, radius, color, canvas)`
Circle outline via a signed-distance-field approach (`apply_aa` on `|distance to radius| - 0.5`), blended with `blend_pixel` for antialiased edges.

`draw_circle_filled(cx, cy, radius, color, canvas)`
Filled circle using the same distance-field technique, filling every pixel whose distance to the center is within the radius.

---

### `rendering/services/effects.rs`

**`blend_pixel(buffer, x, y, width, height, color)`**
Composites a semi-transparent color over the existing pixel using the standard Porter-Duff `over` formula in sRGB space. Reads the background as `[bg_b, bg_g, bg_r, _]` (matching the B,G,R,A buffer layout), blends each channel, and writes back in the same order.

**`draw_gradient_horizontal(x, y, width, height, color_start, color_end, canvas)`**
Fills a rectangle with a left-to-right linear gradient between two colors.

**`draw_vertical_gradient(x, y, width, height, color_start, color_end, canvas)`**
Fills a rectangle with a top-to-bottom linear gradient between two colors.

**`draw_angular_gradiant(x, y, width, height, angle, color_start, color_end, canvas)`**
Fills a rectangle with a gradient along an arbitrary angle in degrees.

**`apply_aa(distance) -> u8`**
Returns an alpha value in `[0, 255]` for a signed distance field input. Used for smooth circle and line edges.

---

### `rendering/services/text/`

The text subsystem loads TTF/OTF fonts (including **variable fonts**), extracts glyph outlines at a given size and weight, and rasterizes them with high-quality anti-aliasing. It is entirely CPU-based and depends only on `ttf-parser` for outline and variation data.

---

**`text/loader.rs`**

`load_font(path: &str) -> Result<Vec<u8>, String>`
Reads a TTF/OTF font file from disk into a byte buffer. The buffer is passed by reference to all subsequent glyph operations. Note: `draw_text` currently calls this (and re-parses the face) once per character — see [§17](#17-current-capabilities-and-limitations) for the performance implication.

---

**`text/glyph.rs`**

`exctract_glyph(font_data: &[u8], character: char, size: f32, weight: f32) -> Result<Glyph, String>`

Extracts a single character's outline from the font, at a given point size and a given position on the font's `wght` variation axis, and returns a `Glyph` ready for rasterization.

**Pipeline:**
1. Parse the font face with `ttf_parser::Face::parse` (mutable — variable fonts need a mutable face to set variation coordinates).
2. Apply the requested weight: `face.set_variation(Tag::from_bytes(b"wght"), weight)`. `ttf_parser` clamps out-of-range values to the font's actual axis min/max automatically, and applies any `avar` axis remapping the font defines. See [§11](#11-variable-font-weight-support) for how to discover a font's real axis range.
3. Look up the glyph id for `character`.
4. Walk the TTF outline commands (`move_to`, `line_to`, `quad_to`, `curve_to`, `close`) using a custom `OutlineBuilder`. Because the face already has the weight variation applied, the outline returned here is the *interpolated* (already-bold-or-thin) shape — `gvar` deltas are applied by `ttf_parser` transparently.
5. Bezier curves are flattened via **recursive De Casteljau subdivision** with a flatness threshold of `0.5` font units — curves split until the maximum deviation of any control point from the chord is below this threshold (capped at 8 levels of recursion). This threshold is applied in raw font-design units (typically 0–1000 or 0–2048 per em), i.e. long before the final pixel scale is known, so in practice it always produces far more segments than a small on-screen glyph needs — safe, but not the most efficient possible choice.
6. Compute `width`, `height`, `advance_width` (from `glyph_hor_advance`, which reflects `HVAR` deltas for the current weight), `descent` (from `-bbox.y_min`, clamped to ≥ 0), and `lsb` (from `bbox.x_min`), all scaled by `size / units_per_em`.
7. Contour points are scaled and shifted so the origin sits at the glyph's own ink bounding-box minimum — i.e. contour space is `[0, width] × [0, height]`, **not** aligned to the font's baseline. The baseline offset is carried separately via `descent`/`lsb` so `draw_text` can position each glyph correctly relative to a shared baseline (see [§12](#12-text-layout-correctness--baseline-side-bearings-minimum-spacing)) without needing to touch the contour data itself.

---

**`text/renderer.rs`**

`draw_glyph(glyph: &Glyph, x: u32, y: u32, color: &Color, canvas: &mut Canvas)`

Rasterizes a `Glyph` onto the canvas at pixel position `(x, y)` (the glyph's own local origin, **not** the baseline — the caller is responsible for baseline math, see §12) using a high-quality scanline algorithm.

**Algorithm:**

1. Coverage buffer width is `width.ceil() + 1` and height is `height.ceil()` (both computed with `.max(1)`) — the `+1` on width reserves room for the antialiased edge column at the right side of the glyph. This matters for anyone computing a glyph's rightmost drawn pixel externally (see [§14.5](#145-off-by-one-in-the-minimum-gap-calculation)).
2. For each pixel row `py` (0 to `height`):
   - Cast **8 vertical sub-scanlines** at positions `py + (s + 0.5) / 8` for `s` in `0..8`.
   - For each sub-scanline, find all x-intersections with the glyph contour edges using the standard half-open interval rule (avoids double-counting shared vertices). Each intersection records a **winding direction** (`+1` or `-1`) based on whether the edge crosses upward or downward through the scanline.
   - Sort intersections by x and walk them left to right, maintaining a running winding-number total. A span between two consecutive intersections is filled **only while the running total is nonzero** — this is the nonzero-winding rule, not even-odd (see [§14.1](#141-holes-at-stroke-junctions-even-odd-vs-nonzero-winding) for why).
   - For each pixel column within a filled span, accumulate the **exact fractional horizontal coverage** of `[px, px+1]` that falls inside the span.
3. Normalize accumulated coverage across 8 sub-scanlines to get a value in `[0, 1]`.
4. Apply **stem darkening**: raise the coverage to the power `STEM_DARKEN_GAMMA` (`0.7`) before using it as alpha, boosting faint low-coverage pixels (thin strokes at small sizes) without touching fully-covered or fully-empty pixels. See [§13](#13-stem-darkening).
5. Multiply by the color's own alpha channel.
6. **Blend in linear light:** convert the foreground color and background pixel from sRGB to linear (reading the background as `[buf[idx], buf[idx+1], buf[idx+2]] = [B, G, R]`, matching the canvas's byte order), apply the Porter-Duff `over` operator, convert back to sRGB, and write back in `[B, G, R, A]` order.

The gamma-correct blend step is the most visually significant baseline decision: blending in perceptually-encoded sRGB space produces edges that are too dark at mid-coverage. Blending in linear space gives edges the correct physical weight — identical in spirit to what FreeType and all modern text engines produce.

---

**`text/kerning.rs`**

`get_advance(glyph: &Glyph) -> f32`
Returns the horizontal advance width of a glyph (cursor movement after drawing). Despite the file name, this does not currently implement pair kerning (GPOS/kern-table lookups) — it's a thin wrapper around `glyph.advance_width`, which already reflects the current variable-font weight via `HVAR`.

---

### `rendering/managers/renderer.rs`

The public API of the rendering module.

`draw_rect(x, y, width, height, color, canvas)`
Delegates to `shapes::rect::draw_rect`.

`draw_text(text: &str, font_path: &str, x: u32, y: u32, size: f32, weight: f32, color: &Color, canvas: &mut Canvas) -> Result<(), String>`

Draws a string of characters at position `(x, y)` in the given font, point size, and position on the `wght` axis. For each character:
1. Extracts the glyph at `(size, weight)`.
2. Computes `glyph_x` from the running cursor plus the glyph's own `lsb`, then enforces a minimum real pixel gap against the previous glyph's actual rightmost painted column (`MIN_PIXEL_GAP = 1`) — see [§12.3](#123-minimum-pixel-gap-enforcement).
3. Computes `glyph_y` from `y + size - glyph.height + glyph.descent`, so every glyph shares one baseline and descenders hang below it — see [§12.1](#121-shared-baseline).
4. Calls `draw_glyph`, then advances the cursor by the glyph's `advance_width`.

See [§12](#12-text-layout-correctness--baseline-side-bearings-minimum-spacing) for the full rationale behind each of these steps — they were not all present in earlier versions of this function, and each one fixes a specific, visually-confirmed bug (documented in [§14](#14-rendering-correctness--bugs-found-and-fixed)).

---

### Correct rendering sequence (from tests)

```rust
let mut canvas = Canvas::new(800, 800);
let red = Color::new(255, 0, 0, 255);
let blue = Color::new(0, 0, 255, 255);

draw_rect(20, 50, 160, 120, &red, &mut canvas);
draw_circle(520, 110, 60, &red, &mut canvas);
draw_gradient_horizontal(220, 430, 180, 100, &red, &blue, &mut canvas);
draw_text("Hello", "src/Sora-VariableFont_wght.ttf", 100, 700, 32.0, 400.0, &red, &mut canvas)?;

unsafe {
    std::ptr::copy_nonoverlapping(
        canvas.buffer.as_ptr(),
        window.ptr(),
        canvas.buffer.len(),
    );
}

attach(window.connection_mut(), surface_id, buffer_id)?;
damage_buffer(window.connection_mut(), surface_id, 0, 0, width, height)?;
commit(window.connection_mut(), surface_id)?;
run_event_loop(&mut window, ...)?;
```

Note the extra `weight` argument (`400.0` above = Regular) compared to earlier versions of this README — see [§11](#11-variable-font-weight-support).

---

## 9. The full window creation sequence

```
connect()                              → opens Unix socket to compositor
get_registry()                         → discovers all available services
find_global("wl_shm/compositor/xdg")  → gets dynamic numeric ids
bind_global("wl_shm", 4)              → activates shm service
create_shared_memory(w*h*4)           → allocates RAM via memfd_create + ftruncate
map_memory(fd)                         → maps RAM into process address space
create_shm_pool(fd, size)             → tells compositor to use our RAM
create_buffer(pool, width, height)    → declares a displayable image (format ARGB8888)
bind_global("wl_compositor", 7)       → activates compositor service
create_surface(compositor)            → creates blank drawing surface
bind_global("xdg_wm_base", 9)        → activates shell service
get_xdg_surface(xdg_wm_base, surf)   → promotes surface to shell management
get_toplevel(xdg_surface)             → declares it a top-level window
commit(surface)                        → triggers configure event from compositor
wait_for_configure(xdg_surface)       → waits for compositor approval
ack_configure(xdg_surface, serial)    → confirms the configuration
damage_buffer(surface, 0, 0, w, h)   → marks full buffer as changed
attach(surface, buffer)               → links pixel data to the surface
commit(surface)                        → displays the window on screen

→ caller draws into a Canvas, copies to window.ptr(), then attach+damage+commit
→ caller invokes run_event_loop() to keep window alive
→ caller invokes unmap_memory() after the loop exits
```

---

## 10. Dependencies

```toml
[dependencies]
libc = "0.2"
ttf-parser = "0.21"
azure-core = { path = "../azure-core" }
```

| Dependency | Purpose |
|---|---|
| `libc` | `memfd_create`, `ftruncate`, `mmap`, `munmap`, `sendmsg`, `CMSG_*`, `SCM_RIGHTS` |
| `ttf-parser` | TTF/OTF font parsing — glyph outline extraction, bounding boxes, advance widths, **and variable-font axis/variation support** (`variable-fonts` cargo feature, enabled by default) |
| `azure-core` | `AzureWindowProvider` trait and `WindowEvent` enum |

No dependency was added to support variable-font weight — `ttf-parser` already covered it (`Face::set_variation`, `Face::variation_axes`). See [§11](#11-variable-font-weight-support).

---

## 11. Variable font weight support

Both fonts shipped with the engine (`Sora-VariableFont_wght.ttf`, `Roboto-VariableFont_wdth,wght.ttf`) are **variable fonts**: a single font file that can interpolate between multiple weights (and, for Roboto, widths) instead of shipping one file per style.

`draw_text` and `exctract_glyph` both take a `weight: f32` parameter on the standard CSS-style scale (`100` = Thin ... `400` = Regular ... `700` = Bold ... `900` = Black), applied via:

```rust
face.set_variation(ttf_parser::Tag::from_bytes(b"wght"), weight);
```

`ttf_parser` clamps out-of-range values to the font's actual axis bounds and applies the font's `avar` table if present, so passing an out-of-range weight is safe but not necessarily meaningful — always check a font's real range before relying on a specific value:

```rust
let face = ttf_parser::Face::parse(&font_data, 0)?;
for axis in face.variation_axes() {
    println!("{:?} min={} default={} max={}",
        axis.tag.to_bytes(), axis.min_value, axis.def_value, axis.max_value);
}
```

Measured axis ranges for the two shipped fonts:

| Font | Axis | Min | Default | Max |
|---|---|---|---|---|
| Sora | `wght` | 100 | 400 | **800** (not 900 — no true Black instance) |
| Roboto | `wght` | 100 | 400 | 900 |
| Roboto | `wdth` | 75 | 100 | 100 (not currently exposed by `draw_text` — width axis is left at its default) |

Weight changes affect more than stroke thickness: `HVAR`-driven advance widths and side bearings change too (heavier weights get proportionally *less* extra spacing relative to their ink width, not more) — this interacts directly with the minimum-gap logic described in [§12.3](#123-minimum-pixel-gap-enforcement).

---

## 12. Text layout correctness — baseline, side bearings, minimum spacing

Three separate, independently-discovered layout bugs were fixed in `draw_text`; together they form the current positioning logic. Each is described in isolation here; the concrete before/after symptoms are in [§14](#14-rendering-correctness--bugs-found-and-fixed).

### 12.1 Shared baseline

Every glyph in a run of text must sit on the same **baseline** — the invisible line that letters like `H`, `e`, `l`, `o` rest on, and that descenders (`g`, `p`, `q`, `j`, `y`) hang below. `Glyph` doesn't store an absolute baseline position (its contours are normalized to its own ink bounding box, `[0, width] × [0, height]`), so `draw_text` reconstructs it per glyph:

```rust
let glyph_y = (y as f32 + size - glyph.height + glyph.descent).round() as u32;
```

`y + size` approximates the shared baseline row for the whole call; subtracting `glyph.height` moves up to the top of this specific glyph's ink; adding back `glyph.descent` lets glyphs whose ink dips below the baseline (a nonzero `descent`) sit that much lower than glyphs that don't. The whole expression is evaluated in `f32` and rounded exactly once, rather than rounding `size` and `glyph.height` independently and subtracting the rounded integers — the latter introduced up to ~1px of per-glyph jitter that read as visually inconsistent letter sizes across a word (see [§14.3](#143-descenders-not-hanging-below-the-baseline)).

### 12.2 Left side bearing (LSB)

A glyph's contours are normalized so its ink starts at local `x = 0` — convenient for rasterization, but it throws away the glyph's natural left margin (`lsb`, stored separately on `Glyph`). `draw_text` adds it back in when positioning:

```rust
let glyph_x = (cursor_x + glyph.lsb).round() as i64;
```

Without this, every glyph is drawn flush against the pen position regardless of its own design margin, and the cursor still advances by the *full* `advance_width` — so different glyphs (which have different natural `lsb`) end up visually squeezed together by inconsistent amounts. See [§14.4](#144-left-side-bearing-collapsed-to-zero).

### 12.3 Minimum pixel gap enforcement

Even with (12.1) and (12.2) correct, two glyphs can still end up touching on screen. The font's own side bearing is a *float* value; once both glyphs' positions are independently rounded to integer pixels for rasterization, a small-but-positive float gap (e.g. `0.4px`) can round away to `0` real pixels of separation — worst at small sizes and heavy weights, where the font's own bearing is already thin relative to the pixel grid (see [§11](#11-variable-font-weight-support)).

The fix tracks, per glyph, the **actual rightmost pixel column `draw_glyph` can paint** (not just the theoretical bearing), and pushes the next glyph's position forward if it would land closer than `MIN_PIXEL_GAP` (currently `1`) real pixels away:

```rust
const MIN_PIXEL_GAP: i64 = 1;
// ...
if let Some(prev_edge) = prev_right_edge {
    let min_x = prev_edge + MIN_PIXEL_GAP;
    if glyph_x < min_x {
        cursor_x += (min_x - glyph_x) as f32;
        glyph_x = min_x;
    }
}
// ...
prev_right_edge = Some(glyph_x + glyph.width.ceil() as i64);
```

This enforcement happens in **rounded pixel space**, deliberately, after the earlier attempt to do it in float space (a proportional `size * 0.06` minimum gap) turned out to still round away to nothing at very small sizes (`size = 8`) — see [§14.5](#145-off-by-one-in-the-minimum-gap-calculation) for the full history, including an off-by-one in the right-edge estimate that initially made even this version ineffective.

This rule is intentionally an absolute pixel guarantee, not a proportional one: proportional gaps looked inconsistent between uppercase and lowercase text at the same size (see [§14.5](#145-off-by-one-in-the-minimum-gap-calculation)), because the same *relative* gap reads very differently against tall vs. short glyphs. A flat pixel minimum reads consistently regardless of case or glyph height.

---

## 13. Stem darkening

Without true font hinting (see [§15](#15-hinting--what-it-is-and-why-were-not-implementing-it)), thin strokes at small sizes only cover a small fraction of a pixel and rasterize as faint gray instead of a crisp line — most visible on thin weights (`wght` near 100) below ~14px. System text renderers compensate with **stem darkening**: artificially boosting low antialiasing-coverage values before blending, so thin strokes stay legible instead of fading out.

The engine's version is a coverage remap applied right before the coverage becomes the blend alpha, in `draw_glyph`:

```rust
const STEM_DARKEN_GAMMA: f32 = 0.7;
// ...
let raw_cov = (cov / AA_SAMPLES as f32).min(1.0);
let boosted_cov = raw_cov.powf(STEM_DARKEN_GAMMA);
let alpha = (fg_a * boosted_cov).min(1.0);
```

An exponent below `1.0` lifts mid/low coverage values up (e.g. `0.25` becomes `~0.38` at `γ = 0.7`) while leaving `0.0` and `1.0` fixed points untouched — so fully-empty and fully-covered pixels are unaffected, only the ambiguous partial-coverage pixels get boosted. Verified visually at `size = 10px, weight = 100` (the faintest realistic case): text goes from washed-out pale green to solidly legible, with no change to letter shapes or the spacing logic in §12.

This is a coverage-space approximation of what real stem darkening does (FreeType's CFF driver computes an actual outline expansion amount from ppem + stem width and grows the geometry before rasterizing — see §15). The gamma-curve version here is far cheaper and has no dependency on stem *detection*, at the cost of being a global approximation rather than a per-stem-width-aware one.

---

## 14. Rendering correctness — bugs found and fixed

This section is a deliberately detailed changelog of every text/color rendering bug found during development, in the order they were found, because several of them look superficially similar but have unrelated root causes. Each entry has: the visible symptom, the root cause, and the fix.

### 14.1 Holes at stroke junctions (even-odd vs. nonzero winding)

**Symptom:** small square holes exactly where two strokes of a glyph meet — the crossbar/stem junction of `t` and `H`, the arm/bowl junction of `e`.

**Root cause:** `draw_glyph`'s scanline fill paired up sorted x-intersections two at a time (`chunks(2)`) and filled between each pair — the even-odd rule. Sora's outlines (like many fonts, especially variable fonts) represent some glyphs as **overlapping contours** rather than one clean non-self-intersecting polygon. Under even-odd, a region covered by two overlapping contours is counted twice and cancels out to "outside," producing a hole exactly where the overlap is densest — the stroke junctions.

**Fix:** track each intersection's winding direction (`+1`/`-1` based on whether the edge crosses the scanline upward or downward) and fill a span only while the running winding-number total is nonzero. This is correct regardless of how many times contours overlap. See the algorithm description in [§8](#textrendererrs).

### 14.2 Bottom row of every glyph clipped

**Symptom:** the very bottom pixel row of every glyph was hard-clipped instead of fading out smoothly — visible as an abrupt, flat cutoff instead of a soft antialiased edge.

**Root cause:** `draw_glyph`'s row loop bound was `glyph.height as u32` — a **truncating** cast. Whenever a glyph's true height had a fractional part (the common case), the loop stopped one row short of the actual bottom of the glyph, permanently skipping the fractional-coverage row that should have contained the bottom edge's antialiasing.

**Fix:** `(glyph.height.ceil() as u32).max(1)` — consistent with how `width` was already handled (`glyph.width.ceil()`).

### 14.3 Descenders not hanging below the baseline

**Symptom:** `g`, `p`, `q`, `j`, `y` sat at the same height as every other letter instead of dipping below the line — and separately, letters looked inconsistently sized/positioned relative to each other.

**Root cause:** two compounding issues in the pre-fix positioning formula, `y + (size as u32 - glyph.height as u32)`:
1. It aligned every glyph's **bounding-box bottom** to the same row, regardless of whether that glyph's bounding box naturally extends below the baseline. Since `Glyph` didn't track where the baseline actually was relative to its own bounding box, there was no way to place descenders correctly — this needed a new field (`descent`, see [§6](#rendering-modelsglyphrs--glyph)).
2. `size as u32` and `glyph.height as u32` were each truncated to an integer **independently** before subtracting, introducing up to ~1px of rounding jitter per glyph — visible as inconsistent apparent letter sizes across a word, even though the underlying scale was identical for every glyph.

**Fix:** added `descent: f32` to `Glyph` (computed as `(-bbox.y_min * scale).max(0.0)` — the ink extent below `y = 0` in font units), and replaced the two-step truncating subtraction with a single `f32` expression rounded once: `(y as f32 + size - glyph.height + glyph.descent).round()`.

### 14.4 Left side bearing collapsed to zero

**Symptom:** inconsistent, occasionally cramped spacing between specific letter pairs, worse at small sizes.

**Root cause:** in `exctract_glyph`, contour points are shifted so the glyph's own ink starts at local `x = 0`:
```rust
((px - x_min) * scale, (py - y_min) * scale)
```
This is necessary for compact rasterization, but it also discards the glyph's true left side bearing (the gap between the pen position and where the ink starts) — every glyph gets flush-drawn against the cursor with zero left margin, while the cursor still advances by the *full* `advance_width` (which does include the intended margin). Different glyphs have different natural `lsb` (Sora's ranges from `24` to `106` font units across common letters — roughly `0.8px` to `3.4px` at 32px), so this produced visibly uneven spacing rather than uniformly-too-tight spacing.

**Fix:** added `lsb: f32` to `Glyph` (`bbox.x_min * scale`), kept the contour-shifting behavior for rasterization efficiency, and had `draw_text` position each glyph at `cursor_x + glyph.lsb` instead of `cursor_x` — while switching the cursor itself from a per-glyph-truncated `u32` accumulator to a running `f32` that's only rounded at the point each glyph is actually drawn (removing a second, independent source of cumulative rounding jitter).

### 14.5 Off-by-one in the minimum-gap calculation

**Symptom:** after implementing (§12.3)'s minimum-gap logic once already, specific letter pairs (`a`+`b`, `h`+`i`+`j`+`k`) still visibly touched or merged at very small sizes (`size = 8`), even though every individual `(lsb, width, advance)` value traced by hand satisfied a ≥1px gap requirement.

**Two root causes, found in sequence:**
1. **First attempt used a proportional gap** (`(size * 0.06).max(0.35)`) instead of an absolute pixel minimum, on the theory that a fixed pixel amount would look "huge" on small glyphs and negligible on large ones. This backfired: at `size = 12`, the proportional minimum came out to `0.72px` — still less than one full pixel, so independent rounding of each glyph's position could still erase it entirely. Small sizes need *more* absolute protection against rounding error, not less; the proportionality intuition was solving a legibility question (§13's actual job) with a spacing mechanism, not a rounding-safety question.
2. **After switching to an absolute integer-pixel-space gap** (§12.3's final version), the touching persisted at `size = 8` specifically. Tracing the exact numbers showed every consecutive pair satisfied the ≥1px rule *by the formula being used* — the bug was in the formula itself: the previous glyph's estimated rightmost painted column was computed as `glyph_x + glyph.width.ceil() - 1`, but `draw_glyph`'s coverage buffer is `width.ceil() + 1` columns wide specifically to leave room for an antialiased edge column at local x = `width.ceil()` — one column further right than the estimate assumed. The "1px gap" being enforced was therefore actually guaranteeing a 0px real gap once that edge column's antialiasing was accounted for.

**Fix:** `prev_right_edge = glyph_x + glyph.width.ceil()` (no `- 1`), matching `draw_glyph`'s actual paintable range exactly. Verified by direct pixel tracing (`/tmp/azure_text_check`-style ad hoc test binaries — see [§16](#16-test-suite-for-text-rendering)) at `size = 8, weight = 100` before and after: before, `h`/`i`/`j`/`k` rendered as an unreadable merged blob; after, every letter is a distinct, separated shape.

### 14.6 Red and blue channels swapped everywhere

**Symptom:** every color drawn as "red" (`Color::new(255, 0, 0, 255)`) — rectangles, lines, and red-tinted text — rendered as **blue** on screen. Not text-specific; affected every shape and gradient in the demo scene.

**Root cause:** `shm_manager::create_buffer` declares the `wl_buffer` format as `ARGB8888` (`wl_shm.format` value `0`), which the Wayland/DRM spec defines as a 32-bit `0xAARRGGBB` value — stored **little-endian in memory as bytes `[B, G, R, A]`**. Every pixel-writing function in the engine (`buffer::set_pixel`, `effects::blend_pixel`, `text/renderer::draw_glyph`) was instead writing `[color.r, color.g, color.b, color.a]` — plain R,G,B,A order. The two byte orders are mirror images of each other in the R/B position, so every red/blue pair silently swapped; green was unaffected because it sits in the same (second) byte position either way.

**Fix:** all three write sites now store `[color.b, color.g, color.r, color.a]`, and everywhere a pixel is *read back* for blending (`blend_pixel`'s background read, `draw_glyph`'s background read for Porter-Duff compositing) reads `buf[idx]` as blue and `buf[idx+2]` as red to match. Verified with a direct assertion: drawing `Color::new(255, 0, 0, 255)` (pure red) now produces the bytes `[0, 0, 255, 255]` in `canvas.buffer` — blue channel `0`, green `0`, red `255`, alpha `255`, exactly `ARGB8888`'s expected memory layout for pure red.

This bug predates the text-rendering work in this changelog and would have affected the engine's very first rectangle/line/gradient output — it was only found while investigating an unrelated text color report, by cross-checking a full-window screenshot against the demo scene's known `Color::new(255, 0, 0, 255)` calls.

### 14.7 Crude coordinate snapping — tried, rejected

Not a bug fix — an experiment, kept here because it's useful negative information. To test whether a cheap approximation of hinting would help legibility further, `tests/text_snapping_experiment.rs` rounds every contour point of a glyph to the nearest integer pixel *after* scaling, with no stem detection or geometric awareness, and compares the result against the normal renderer side by side.

**Result:** at `12px / weight 100` the snapped version was *more* broken than doing nothing — visible holes and illegible letterforms, worse than the unsnapped baseline. At `12px / weight 400` it was rougher with no clear benefit. At `24px / weight 400` the two were roughly equivalent. Rounding every point independently, with no understanding of which points belong to the same stem or curve, distorts curves and can make one side of a symmetric letter (like `o`) round differently from the other.

**Conclusion:** rejected. Not wired into `draw_glyph` or `draw_text`. Real hinting (§15) requires understanding *which* points form a stem before deciding how to snap them — naive per-point rounding is not a shortcut around that requirement.

---

## 15. Hinting — what it is, and why we're not implementing it

"Hinting" (a.k.a. grid-fitting) is the general term for adjusting a glyph's outline, at rasterization time, so its important features — stem widths, baseline, x-height, cap-height — land on exact pixel boundaries instead of wherever the raw scaled outline happens to fall. It's why system-rendered small text (in a browser, an IDE, a terminal) looks crisper than a naive scale-and-rasterize of the same outline, which is what this engine currently does.

There are two distinct mechanisms in real-world font renderers (FreeType, DirectWrite, CoreText), of very different scope:

**TrueType bytecode instructions** — some fonts embed literal programs (`fpgm`/`prep`/per-glyph instructions) that a virtual machine executes at rasterization time to hint that specific font's specific glyphs. Implementing this means implementing a full instruction-set interpreter (~dozens of opcodes, a "twilight zone" concept, storage/control-value tables). Out of scope: it's a large, self-contained subsystem, and many modern webfonts (including both fonts shipped here) don't ship meaningful bytecode instructions anyway — they rely on the next mechanism instead.

**Autohinting** — analyzes *any* outline geometrically (no embedded instructions required): detect stem segments, establish "blue zones" from reference letters (`o`, `H`, `x`, etc.) for baseline/x-height/cap-height alignment, then warp the outline so those features snap to the pixel grid, consistently across the whole glyph. This is the mechanism that would actually help our two variable fonts. It's also a multi-thousand-line subsystem in FreeType (`aflatin.c`, `afhints.c`, `afglobal.c`, ...) — a multi-week undertaking with high risk of subtle, hard-to-debug visual regressions if attempted incrementally.

A **crude approximation** (round every already-scaled contour point to the pixel grid, no stem detection) was tried as a bounded experiment and rejected — see [§14.7](#147-crude-coordinate-snapping--tried-rejected). It reliably made small/thin text *worse*, not better.

**Where things stand:** real hinting is left as future work (a good candidate for a from-scratch, deliberately scoped implementation — the smallest useful slice is probably just blue-zone baseline/x-height snapping, without full stem detection). In the meantime, [stem darkening](#13-stem-darkening) covers the specific "thin strokes fade to gray" symptom cheaply, and the [minimum pixel gap](#123-minimum-pixel-gap-enforcement) covers the "letters touch" symptom — between the two, small-size legibility is significantly better than an unmitigated scale-and-rasterize, without taking on hinting's implementation risk.

If revisited, **FreeType** itself (LGPL/FTL-licensed, the reference implementation used by effectively all of Linux) is the natural reference to study — but pulling it in as a dependency (vs. reading its algorithms and reimplementing them) would mean FFI to a C library, which conflicts with this engine's from-scratch, minimal-dependency approach (see [§1](#1-what-is-azure-engine)).

> **Planned:** the project owner intends to implement this — the real autohinting algorithm, done properly (stem detection, blue zones, coherent outline warping) — by hand, as a deliberate personal challenge, rather than adopting FreeType or leaving it unimplemented. Not scheduled yet; noted here so the intent isn't lost.

---

## 16. Test suite for text rendering

A number of standalone integration tests were built specifically to validate the text pipeline in isolation, independent of any single visual bug report. They live in `tests/` at the crate root (sibling to `src/`) and are ordinary `cargo test` targets. Several are designed to be inspected visually rather than asserted against, and are meant to be kept and rerun rather than deleted after use:

| Test file | What it checks | How to inspect |
|---|---|---|
| `text_min_spacing_check.rs` | Minimum-gap enforcement across size/weight combinations, headless (no Wayland) | `cargo test --test text_min_spacing_check -- --nocapture`, then open the `.ppm` files under `target/tmp/azure_text_check/` |
| `text_min_spacing_window.rs` | Same, but in a real Wayland window for live/visual inspection | `cargo test --test text_min_spacing_window -- --nocapture` |
| `text_alignment_grid.rs` | Draws a line through the center of each glyph's advance cell, overlaid on the real text, to check per-glyph centering without guessing | `cargo test --test text_alignment_grid -- --nocapture`, inspect `target/tmp/azure_text_check/grid_*.ppm` |
| `text_file_vs_window.rs` | Writes the *same* canvas buffer to a file and to a live window in one run, to isolate rendering bugs from display/compositor artifacts | `cargo test --test text_file_vs_window -- --nocapture`, compare the `.ppm` against a screenshot of the window |
| `text_roboto_size_weight_matrix.rs` | Full size (10–24px) × weight (100/400/700/900) matrix on Roboto, live window | `cargo test --test text_roboto_size_weight_matrix -- --nocapture` |
| `text_snapping_experiment.rs` | The rejected crude-snapping experiment from [§14.7](#147-crude-coordinate-snapping--tried-rejected), kept for reference — **not** wired into the real renderer | `cargo test --test text_snapping_experiment -- --nocapture`, inspect `target/tmp/azure_text_check/snap_compare.ppm` |
| `debug_case_compare.rs` | Ad hoc upper/lowercase spacing comparison at multiple sizes | `cargo test --test debug_case_compare -- --nocapture` |

Since `.ppm` isn't always recognized by a default image viewer, convert with ImageMagick if needed: `convert foo.ppm foo.png` (or `magick foo.ppm foo.png` on newer ImageMagick installs).

The headless (`.ppm`-writing) tests exist specifically because, at one point during development, a live-window screenshot appeared to show a bug that a byte-identical headless render of the same draw calls did not — `text_file_vs_window.rs` was built to make that comparison directly reproducible rather than argued about from separate screenshots.

---

## 17. Current capabilities and limitations

**What works:**
- Persistent, interactive window on any Linux Wayland desktop
- Dynamic compositor service discovery
- Shared memory allocation and pixel-level access
- File descriptor transmission via `SCM_RIGHTS`
- Full event loop: ping/pong, configure/ack, clean close
- Monitor resolution query (`get_screen_resolution`)
- Window title (`xdg_manager::set_title`)
- `AzureWindowProvider` trait implemented — `render(pixels)` and `poll_event()`
- **CPU software renderer — fully operational:**
  - `Canvas` — drawing surface with flat pixel buffer, stored directly in Wayland's `B,G,R,A` byte order
  - `Color` — RGBA color model at the API surface (reordered only at write time)
  - `set_pixel` / `get_pixel_index` — pixel-level access with bounds checking
  - `draw_rect` — filled rectangle
  - `draw_rect_rounded` — rectangle with rounded corners
  - `draw_line_horizontal` / `draw_line_vertical` — axis-aligned lines
  - `draw_line` — general line, antialiased
  - `draw_circle` — antialiased circle outline
  - `draw_circle_filled` — filled circle
  - `blend_pixel` — Porter-Duff alpha compositing
  - `draw_gradient_horizontal` / `draw_vertical_gradient` — linear gradients
  - `draw_angular_gradiant` — gradient along an arbitrary angle
  - `draw_text` — high-quality **variable-font-aware** TTF text rendering: nonzero-winding fill, gamma-correct AA, shared baseline with correct descenders, corrected side bearings, guaranteed minimum inter-glyph pixel gap, and stem darkening for small/thin legibility

**Current limitations:**
- No real font hinting (see [§15](#15-hinting--what-it-is-and-why-were-not-implementing-it)) — stem darkening and minimum-gap enforcement mitigate the worst symptoms but don't replace true grid-fitting
- No pair kerning (GPOS/kern table) — `text/kerning.rs` currently only returns the glyph's own advance width
- `draw_text` re-parses the whole font face **per character** (via `exctract_glyph` → `load_font`/`Face::parse`) rather than once per call — correctness is unaffected but this is wasted work on longer strings
- Width axis (`wdth`) on Roboto is not exposed by `draw_text` — only `wght` is settable
- Object ids in `window_manager.rs` partially hardcoded
- No keyboard/mouse input routing yet
- No dynamic resize

---

## 18. What comes next

**Input routing:**
- Bind `wl_seat`, parse keyboard and pointer events
- Return as `WindowKeyPress`, `WindowMouseMove` variants from `poll_event()`

**Dynamic resize:**
- Recreate shared memory and buffer on `WindowResize` event

**Text rendering:**
- Real hinting, scoped down to blue-zone baseline/x-height snapping first (see [§15](#15-hinting--what-it-is-and-why-were-not-implementing-it))
- Pair kerning (GPOS)
- Cache parsed `ttf_parser::Face` per `(font_path, weight)` instead of re-parsing per character

**Future platforms:**
- `win32/` — `AzureWindowProvider` implementation using the Win32 API (CreateWindow, GDI shared memory)
- `cocoa/` — `AzureWindowProvider` implementation using Cocoa / Core Graphics on macOS

All platform-specific code will live under `src/platform/<target>/` behind the same `AzureWindowProvider` trait, keeping Foundation and all applications fully portable.

---

*Azure Engine is built entirely from scratch, without any GUI framework or Wayland binding library. Every byte of every Wayland message is constructed manually. The rendering module follows the same philosophy: no graphics library, pure CPU rasterization from first principles — including its font rendering, which parses and rasterizes TTF/OTF outlines (variable fonts included) without FreeType or any other font-rendering library.*
