# Tungstengine

A 2D and 3D game engine in Rust, built on wgpu and winit.

## Features

- Small ECS engine with resources and hierarchy
- 2D sprite renderer and 3D renderer with shadows, post-processing and skeletal animation
- Immediate-mode UI with widgets, tooltips and pause, settings and controls menus
- Input actions with player rebinding, gamepads and multiple players
- Localization from TOML files
- Scenes and prefabs, tilemaps with layers, autotiling and dual grid
- Save and settings storage on desktop and web
- Yarn Spinner dialog
- Web builds through wasm

## Crates

| Crate | Purpose |
|---|---|
| `tungstengine` | The engine |
| `tungstengine-editor` | Editor shell and 3D scene editor |
| `ecs_macros` | Derive macros |

## Feature flags

| Flag | Enables |
|---|---|
| `2d` | 2D renderer and runtime |
| `3d` | 3D renderer, runtime, physics and navigation |
| `scene` | Scenes, prefabs and tilemap spawning |
| `dialog` | Yarn Spinner dialog |

## Usage

```toml
[dependencies]
tungstengine = { git = "https://github.com/wbmagalhaes/tungstengine", default-features = false, features = ["2d"] }
```

A game has a `manifest.toml` at its root:

```toml
title = "My Game"
width = 1280.0
height = 720.0
locale = "en"
```

Translations live in `assets/locale/<locale>.toml`.

```rust
use tungstengine::prelude::*;
use tungstengine::runtime::d2::run;

#[derive(Action, Clone, Copy)]
enum GameAction {
    Jump,
}

fn setup(world: &mut World) {
    tungstengine::boot!(world);
    let mut actions = Actions::default();
    actions.declare(GameAction::Jump, &[Binding::Key(KeyCode::Space)]);
    world.load_actions(actions);
}

fn frame(world: &mut World, ui: &mut UI) {
    if world.input().pressed(GameAction::Jump) {
        log::info!("jump");
    }
    ui.add(Text::new(16.0, 16.0, "Hello"));
}

fn main() -> SysResult {
    run(setup, frame)?;
    Ok(())
}
```

## Web builds

Run `scripts/build-web.sh` from the game folder. It needs `wasm-bindgen-cli`. The page comes from `web/` and a game can replace `index.html`, `style.css` or `wrapper.html` in its own `web/` folder, using `{{game}}` to place the canvas.

## Development

```sh
make check
```

## License

MIT, see [LICENSE](LICENSE). The bundled fonts keep their own licenses, see [assets/fonts/licenses](assets/fonts/licenses).
