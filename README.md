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

const GRAVITY: f32 = 1800.0;
const JUMP_SPEED: f32 = 700.0;
const SIZE: f32 = 64.0;

#[derive(Action, Clone, Copy)]
enum GameAction {
    Jump,
}

#[derive(Resource, Default)]
struct Player {
    height: f32,
    speed: f32,
}

fn setup(world: &mut World) {
    tungstengine::boot!(world);
    let mut actions = Actions::default();
    actions.declare(GameAction::Jump, &[Binding::Key(KeyCode::Space)]);
    world.load_actions(actions);
    world.insert_resource(Player::default());
}

fn frame(world: &mut World, ui: &mut UI) {
    let jump = world.input().pressed(GameAction::Jump);
    let dt = world.dt();
    let Ok(player) = world.resource_mut::<Player>() else {
        return;
    };
    if jump && player.height <= 0.0 {
        player.speed = JUMP_SPEED;
    }
    player.speed -= GRAVITY * dt;
    player.height = (player.height + player.speed * dt).max(0.0);
    if player.height <= 0.0 {
        player.speed = 0.0;
    }

    let ground = ui.height - 120.0;
    ui.fill(Rect::new(0.0, ground, ui.width, 120.0), Color::GRAY_700);
    let x = (ui.width - SIZE) / 2.0;
    let y = ground - SIZE - player.height;
    ui.fill(Rect::new(x, y, SIZE, SIZE).with_z(1.0), Color::AMBER_300);
    ui.add(Text::new(16.0, 16.0, "Press Space to jump"));
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
