# Ely GPUI Component

A component library for [GPUI](https://www.gpui.rs), the Rust UI framework behind Zed.
Every component ships in light and dark. The palette is warm and quiet. Color carries meaning, not decoration.

Every component runs live at https://elygpui.com, compiled to WebAssembly.

Status: early. `TASKS.md` tracks every component, chapter by chapter. To help, see `CONTRIBUTING.md`.

**This fork** (branch `sjel`) builds on [GPUI Kit](https://gpui-kit.com) 0.10.0 instead of a
direct `gpui-pre` pin, so it follows whichever engine the application selects: `gpui-pre`
by default, GPUI Fast with the `gpui-fast` feature. It is maintained for
[Sjel](https://github.com/larsboes/Sjel); upstream is
[ZacharyZhang-NY/Ely-GPUI-Components](https://github.com/ZacharyZhang-NY/Ely-GPUI-Components).

## Use

```toml
[dependencies]
ely-gpui-component = { git = "https://github.com/larsboes/Ely-GPUI-Components", branch = "sjel" }
gpui-kit = { version = "=0.10.0", default-features = false }
```

```rust
use ely_gpui_component::{Assets, theme::{Mode, Theme}};
use gpui_kit::App;

fn main() {
    gpui_kit::platform::application().with_assets(Assets).run(|cx: &mut App| {
        ely_gpui_component::init(cx).expect("Ely failed to start");
        Theme::set_mode(Mode::Dark, cx);
    });
}
```

`init` registers the fonts and the theme. It returns an error if the application's asset source cannot load one of Ely's files.

## With GPUI Kit

This fork depends on GPUI Kit 0.10.0 and reuses its selected engine. To use Kit's components too, keep the app on that Kit version, enable its default features, pass `Assets::before(gpui_kit::assets::Assets)` as the one asset source, and call both `init`s. Open the window with `gpui_kit::open_window` and wrap the view in Ely's root `FocusScope`. The [guide](frontend/src/docs/gpui-kit.md) covers engine selection, themes, focus and the inspector; `compat/kit` holds a whole app and integration tests.

## Gallery

```sh
cargo run --example gallery
cargo run --example gallery -- --page buttons
cargo run --example gallery -- --capture shots   # macOS: PNG of every page, light and dark
ELY_GALLERY_ASSETS=https://example.com/gallery/assets/ scripts/web.sh dist   # for the browser
```

In a browser, `?page=buttons&story=icon-button&theme=dark` draws one section alone; `examples/gallery/stories.json` lists them all.

## Website

```sh
cd frontend
pnpm install
ELY_GALLERY_ASSETS=https://<site>/gallery/assets/ pnpm gallery   # the wasm gallery, into public/gallery
pnpm dev        # or: pnpm build && pnpm preview; both fetch the showcase at showcase.lock's commit
ELY_SHOWCASE=../../Ely-GPUI-Showcases pnpm dev   # the showcase from a local checkout
node scripts/og.mjs   # with pnpm preview up: public/og.png from the hero
node scripts/favicon.mjs   # public/favicon.svg, apple-touch-icon.png and ../logo.png from the mark
pnpm deploy     # Cloudflare, through wrangler
```

## Build notes

- Kit enables `runtime_shaders` and `font-kit` on its native platform dependency: Metal shaders compile at runtime without a full Xcode install, and macOS text has its font backend. Ely has no separate `runtime_shaders` feature.
- CI tests the crate and builds the gallery on macOS, Windows and Linux, arm64 and x86_64. The capture tool needs macOS.

## License

MIT. Lucide icons: ISC. Inter, JetBrains Mono, IBM Plex Sans and the gallery's Noto Sans Hebrew: SIL Open Font License 1.1.
The gallery photos in `examples/gallery/assets` were generated for this project with gpt-image-2.
