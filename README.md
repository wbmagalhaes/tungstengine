# Tungstengine

A 2D and 3D game engine in Rust, built on wgpu and winit.

## Features

- Small ECS engine with resources and hierarchy
- 2D sprite renderer and 3D renderer with shadows, post-processing and skeletal animation
- Immediate-mode UI with widgets, tooltips and pause, settings and controls menus
- Input actions with player rebinding, gamepads and multiple players
- Localization from TOML files
- Minimal 2D and 3D physics: boxes, circles, gravity, tile collision and raycasts
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
| `3d` | 3D renderer and runtime |
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

const JUMP_SPEED: f32 = 700.0;
const SIZE: f32 = 64.0;
const GROUND_Y: f32 = 560.0;

#[derive(Action, Clone, Copy)]
enum GameAction {
    Jump,
}

#[derive(Component)]
struct Player;

fn setup(world: &mut World) {
    tungstengine::boot!(world);
    let mut actions = Actions::default();
    actions.declare(GameAction::Jump, &[Binding::Key(KeyCode::Space)]);
    world.load_actions(actions);
    world.insert_resource(Physics2D::with_gravity(Vec2::new(0.0, 1800.0)));

    let player = world.spawn();
    world.insert(player, Player);
    world.insert(player, Transform::from_pos(Vec3::new(640.0, 300.0, 0.0)));
    world.insert(player, Collider2D::rect(Vec2::splat(SIZE)));
    world.insert(player, Body2D::default());

    let ground = world.spawn();
    world.insert(ground, Transform::from_pos(Vec3::new(640.0, GROUND_Y + 80.0, 0.0)));
    world.insert(ground, Collider2D::rect(Vec2::new(4000.0, 160.0)));

    world.add_system(jump);
    world.add_system(physics2d_step);
}

fn jump(world: &mut World) -> SysResult {
    let pressed = world.input().pressed(GameAction::Jump);
    for (_, body, _) in world.query_mut::<(&mut Body2D, &Player)>() {
        if pressed && body.on_ground {
            body.velocity.y = -JUMP_SPEED;
        }
    }
    Ok(())
}

fn frame(world: &mut World, ui: &mut UI) {
    ui.fill(Rect::new(0.0, GROUND_Y, ui.width, ui.height), Color::GRAY_700);
    for (_, t, _) in world.query::<(&Transform, &Player)>() {
        let (x, y) = (t.position.x - SIZE / 2.0, t.position.y - SIZE / 2.0);
        ui.fill(Rect::new(x, y, SIZE, SIZE).with_z(1.0), Color::AMBER_300);
    }
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
