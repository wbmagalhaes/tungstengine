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

`Physics2D::earth()` gives a gravity that feels right at the default scale, and `physics.jump_speed(height)` turns a jump height into a speed. Bodies only turn when built with `.rotating()`, and colliders follow the `Transform` rotation.

```rust
let body = Body2D::default().friction(0.4).sleepy();
let wall = Collider2D::between(min, max);
let trigger = Collider2D::circle(1.0).offset(offset).sensor();
```

Shapes are `rect`, `circle`, `polygon` and `regular`. For concave shapes, or a body made of several parts, attach colliders as child entities; mass and inertia come from the parts, and `body_of` finds the body from a part:

```rust
let boat = world.spawn();
world.insert(boat, Transform::default());
world.insert(boat, Body2D::default().rotating());
world.attach_colliders(boat, Collider2D::concave(&points));
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

## UI style

`Style` has `border_radius` and `border`, drawn like CSS: a signed distance in the shader gives antialiased corners at any size, a border follows the curve, and radii that do not fit shrink together. Use `.border_radius(8.0)`, `.border_radius_t`, `.border_radius_tl` and the other sides and corners, `.border_radius_corners(tl, tr, br, bl)`, `.border_size(2.0)` and `.border_color(color)`. `.outline(color, px)`, `.outline_size`, `.outline_color` and `.outline_offset` draw a ring outside the border, with a gap that shows what is behind. `.outline_sweep(color)` runs a conic gradient of that color around the ring, and `.outline_sweep_speed(turns_per_second)` sets its pace. `.scale(s)`, `.translate(offset)`, `.translate_xy(x, y)`, `.translate_x`, `.translate_y` and `.rotate(radians)` transform a button around its center, text included, and clicks still use its original rect. Buttons grow a little on hover and shrink when pressed by default; set `scale` in `.hover` or `.active` to change that. `hover` and `active` are states, not part of a style: `.hover(style)` and `.active(style)` take a `Style` that applies over the normal one, like `:hover` and `:active`. Unset, the radius falls back to `ui.theme.border_radius`, which also rounds widgets without a style. `.backdrop_blur(px)` blurs the game behind any surface, like `backdrop-filter`; frames that use it pay for extra full-screen passes, and the rest pay nothing. Panels clip their children to the rounded shape, like `overflow: hidden`, and `ui.push_clip_round(rect, radius)` does it by hand. `ui.fill_round` and `ui.fill_bordered` draw by hand. The default of 0 keeps square corners.

`.gap(4.0)` sets both axes, and `.gap_xy`, `.gap_x` and `.gap_y` set them apart; `.scale` works the same. `.pad(4.0)` sets every side, `.pad_tb` and `.pad_lr` set pairs, `.pad_t`, `.pad_r`, `.pad_b` and `.pad_l` set one, and `.pad_sides(top, right, bottom, left)` follows the CSS order. `.gradient(angle, from, to)` is a `linear-gradient`, `.gradient_via(angle, from, via, to)` adds a middle color, with 0 pointing up and PI pointing down. Each widget has its own default transitions, and a state can set its own: `.transition(Transition::new(0.15, Ease::OutCubic).exit(0.3, Ease::InOutQuad))` uses one curve going in and another coming out, and without `.exit` both are the same. A change halfway starts from the current value. Custom widgets can use `ui.interact_curved(rect, id, curve)` for their own hover curve. Every field of `Style` is optional and `None` inherits. Widgets resolve it into a `ComputedStyle`, with no options left, from theme to widget style, and widgets with states apply `hover` and `active` on top, the same way CSS goes from declared to computed values.

`Slider` edits 0 to 1 by default; `.range(min, max)` and `.step(n)` use real units, and `.readout(text)` shows the value however the game wants.

`TextInput` and `TextArea` share one editing core: selection, word jumps, double and triple click, drag and clipboard work the same in both, and a line scrolls when it outgrows its box. `ui.icon_line` draws the small plus, minus, cross and chevron icons.

`Switch` is an on and off toggle with a sliding knob, next to `Checkbox` for lists of choices.

`Tabs` is a segmented control with a sliding fill, `Stepper` lights its pills one by one.

`Divider::horizontal(x, y, length)` and `Divider::vertical` draw a line that fades out, `.solid()` keeps it even, and `.label(text)` puts a title before a horizontal line.

Also in `ui`: `ProgressBar`, `Spinner`, `Badge`, `IconButton`, `Dialog`, `Toasts`, `ContextMenu`, `ScrollView` (vertical, horizontal or both with `.axis`, Shift and the wheel scroll sideways), `Accordion`, `RadioGroup`, `Slot` and `KeybindInput`. Pointer checks respect the current clip, so widgets scrolled out of view take no clicks.

`menus::language_tabs`, `language_button` (sets one locale and shows its `flag-<locale>` sprite; `pt-BR` and `en-US` come with the engine, a game adds others by inserting that sprite) and `language_dropdown` switch the language.

A locale file can set `[language] name = "..."` to name itself; `pt-BR` and `en-US` have built-in names.

## UI lists

Widgets get their identity from their label, so repeated labels need a key. Debug builds warn when a label repeats in a frame.

```rust
for item in &shop {
    ui.scope(item.id, |ui| ui.button(rect, "Buy"));
}
```

## Testing

`runtime::d2::Headless` and `runtime::d3::Headless` run a full game without a window, so tests can press keys, advance frames and read the world and on-screen text.

## Conversations

`dialog::conversation(world, ui)` draws the `Dialog` resource as a box at the bottom of the screen, with the speaker, typed text and choices. A click, Space, Enter or the gamepad South button finishes the line, then advances. Choices sit above the box, numbered: up and down, the d-pad or the left stick move the focus, Space, Enter or South picks, and a number key picks directly. The speaker in Yarn is an actor id, never shown: the name comes from `actor.<id>.name`, or `actor.<id>.unknown` until `<<reveal act:id>>`, in the locale files, falling back to the actor's fields. The portrait is the sprite in the actor's `portrait` field, or `actor-<id>`; `<<mood happy>>` in Yarn uses `actor-<id>-happy` when it exists, and `portrait_side` or `<<side right>>` moves it. Both last for the next line only. Typing waits after punctuation followed by a space: a little after commas, longer after `.`, `?` and `!`. While the text types, the speaker mumbles: each vowel typed plays a synthesized vowel sound, touching vowels glide as one sound, consonants stay silent, except `m` and `n` in words without vowels, like `Hmm`, which hum, and `s`, `h`, `x`, `z` and `f` in the others, like `Shh`, which hiss, shaped by the actor's `voice` and the sfx volume: `pitch`, `variation`, `tract` (above 1 moves the vowel curves up, like a smaller throat) and `brightness` (above 0 favors high partials, below 0 sounds muffled) and `roughness` (0 to 1, how much pitch, level and breath drift, so no two vowels come out identical). Pitch and `tract` together set the speaker's size: `pitch = 0.7, tract = 1.0` reads as a man, `pitch = 1.3, tract = 1.17` as a woman, `pitch = 1.8, tract = 1.3` as a child. The mumble follows the punctuation: pitch falls along each phrase and resets at a comma, `!` starts higher, `?` lifts the last word and `...` fades out. `[shout]` and `[whisper]` change the voice and the look of the text they wrap. `Dialog::from_script(name, source)` loads Yarn from a string. To translate, tag each line in Yarn with `#line:id` and pass a TOML table of `id = "speaker: text"` to `Dialog::with_translation(locale, table)`; conversations follow the current locale, and lines without a translation stay as written.

## Rich text

`RichText` takes inline tags: `[b]`, `[i]`, `[color=#hex]`, `[size=20]`, `[wave]`, `[shake]`, `[rainbow]`, `[pulse]` and `[icon=sprite]`. Conversations use it, and add `[pause=0.5/]` and `[speed=2]...[/speed]` for pacing. In Yarn, write tags that do not wrap text with `/]`, and colors without `#`, like `[color=b388ff]`, because `#` starts a line tag there.

## Web builds

Run `scripts/build-web.sh` from the game folder. It needs `wasm-bindgen-cli`, and uses `wasm-opt` when installed. The page comes from `web/` and a game can replace `index.html`, `style.css` or `wrapper.html` in its own `web/` folder, using `{{game}}` to place the canvas.

## Development

```sh
make check
```

## License

MIT, see [LICENSE](LICENSE). The bundled fonts keep their own licenses, see [assets/fonts/licenses](assets/fonts/licenses).
