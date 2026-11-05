# Tungstengine

A 2D and 3D game engine in Rust, built on wgpu and winit.

## Features

- Small ECS engine with resources and hierarchy
- 2D sprite renderer and 3D renderer with shadows, post-processing and skeletal animation
- Immediate-mode UI with widgets, tooltips and pause, settings and controls menus
- Input actions with player rebinding, gamepads and multiple players
- Localization from TOML files
- 2D and 3D physics: impulse solver with soft contacts, body-to-body collision, mass, friction, sensors, sleeping, tile collision and raycasts
- Particles: declarative effects, emitters as entities, world and screen space, loadable from TOML
- System stages, so physics and camera run in a fixed order
- Optional pixel-perfect mode for pixel art games
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
    let actions = Actions::default().declare(GameAction::Jump, &[Binding::Key(KeyCode::Space)]);
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
    world.add_physics2d();
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

## Systems and stages

`world.add_system` runs in `Stage::Update`. Use `add_system_to` for the others, which always run in this order, whatever order they were added: `PreUpdate`, `Update`, `Physics`, `AfterPhysics`, `Late`.

```rust
world.add_system_to(Stage::PreUpdate, read_input);
world.add_system(movement);
world.add_physics2d();
world.add_system_to(Stage::AfterPhysics, pickups);
world.add_system_to(Stage::Late, camera_follow);
world.add_particles();
```

`add_physics2d_if` and `add_system_to_if` take a condition such as `in_state(Screen::Game)`. `query_mut` accepts any mix of `&T` and `&mut T`.

## Physics

`Physics2D::earth()` gives a gravity that feels right at the default scale, and `physics.jump_speed(height)` turns a jump height into a speed. Bodies do not rotate.

```rust
let body = Body2D::default().friction(0.4).sleepy();
let wall = Collider2D::between(min, max);
let trigger = Collider2D::circle(1.0).offset(offset).sensor();
```

Sensors report through `overlaps` and block nothing. Sleeping is opt-in with `.sleepy()`: a resting body costs almost nothing and wakes when hit or when its velocity is set. 3D has the same API with `Body3D`, `Collider3D` and `add_physics3d`.

## Particles

Effects are plain data with a builder, and can be loaded with `ParticleEffect::parse` from TOML.

```rust
let sparkle = ParticleEffect::new()
    .count(18)
    .life((0.4, 0.8))
    .speed((3.0, 7.0))
    .sprite(Sprite::CAPSULE)
    .align();

emit_burst(world, pos, sparkle.clone());
world.insert(player, ParticleEmitter::stream(dust).offset(feet));
ui.burst(&sparkle, (x, y), z);
```

`emit_burst` and `ParticleEmitter` work in the world and need `world.add_particles()`. `ui.burst` works in screen pixels and needs no system.

## Sprites

`ui.sprite(rect, GameArt::Chest, color)` takes a sprite enum, a name or a `Sprite`. Use `ui.art(key)` or `world.art(key)` to get a `SpriteDraw`. Each `World` resolves its own sprites, and missing ones draw as a magenta tile.

Set `pixel_perfect = true` in `manifest.toml` for pixel art: sprites land on whole pixels and scale by whole numbers. Without it, sprites scale and filter smoothly.

## UI lists

Widgets get their identity from their label, so repeated labels need a key. Debug builds warn when a label repeats in a frame.

```rust
for item in &shop {
    ui.scope(item.id, |ui| ui.button(rect, "Buy"));
}
```

## Testing

`runtime::d2::Headless` and `runtime::d3::Headless` run a full game without a window, so tests can press keys, advance frames and read the world and on-screen text.

## Web builds

Run `scripts/build-web.sh` from the game folder. It needs `wasm-bindgen-cli`, and uses `wasm-opt` when installed. The page comes from `web/` and a game can replace `index.html`, `style.css` or `wrapper.html` in its own `web/` folder, using `{{game}}` to place the canvas.

## Development

```sh
make check
```

## License

MIT, see [LICENSE](LICENSE). The bundled fonts keep their own licenses, see [assets/fonts/licenses](assets/fonts/licenses).
