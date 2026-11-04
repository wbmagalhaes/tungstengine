use std::collections::BTreeMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};

#[cfg(feature = "scene")]
use crate::ecs::prelude::*;
#[cfg(feature = "scene")]
use crate::ecs::set_parent;
use crate::grid::{
    DUAL_FULL, corner_index, dual_cells, neighbor_index, neighbor8_index, nine_index,
};
#[cfg(feature = "scene")]
use crate::scene::{GameObjDef, Prefabs, SpawnRegistry, spawn_object};

const EMPTY: char = '.';
pub const BEHIND_ORDER: i32 = 1000;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TilePlane {
    #[default]
    Xy,
    Xz,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AutoTile {
    #[default]
    None,
    Neighbors4,
    Neighbors8,
    Nine,
    Dual,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TileCollision {
    #[default]
    Pass,
    Block,
    Behind,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TilePlacement {
    pub pos: Vec3,
    pub sprite: Option<String>,
    pub prefab: Option<String>,
    pub layer: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct TileDef {
    pub auto: AutoTile,
    pub fill: Option<String>,
    pub fill_layer: i32,
    pub skip_full: bool,
    pub seam: Option<String>,
    pub seam_sprite: Option<String>,
    pub sprite: Option<String>,
    pub prefab: Option<String>,
    pub layer: i32,
    pub collision: TileCollision,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct TileLayer {
    pub name: String,
    pub rows: Vec<String>,
    pub order: i32,
    pub elevation: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct TileObject {
    pub pos: [f32; 2],
    pub sprite: Option<String>,
    pub prefab: Option<String>,
    pub order: i32,
    pub elevation: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Tilemap {
    pub origin: [f32; 2],
    pub cell: f32,
    pub plane: TilePlane,
    pub empty: TileCollision,
    pub tiles: BTreeMap<String, TileDef>,
    #[serde(rename = "layer")]
    pub layers: Vec<TileLayer>,
    #[serde(rename = "object")]
    pub objects: Vec<TileObject>,
}

impl Default for Tilemap {
    fn default() -> Self {
        Tilemap {
            origin: [0.0, 0.0],
            cell: 1.0,
            plane: TilePlane::default(),
            empty: TileCollision::Pass,
            tiles: BTreeMap::new(),
            layers: Vec::new(),
            objects: Vec::new(),
        }
    }
}

impl TileLayer {
    pub fn at(&self, x: usize, y: usize) -> Option<char> {
        self.rows.get(y)?.chars().nth(x)
    }

    pub fn set(&mut self, x: usize, y: usize, tile: char) -> bool {
        let Some(row) = self.rows.get_mut(y) else {
            return false;
        };
        let mut chars: Vec<char> = row.chars().collect();
        match chars.get_mut(x) {
            Some(c) if *c != tile => *c = tile,
            _ => return false,
        }
        *row = chars.into_iter().collect();
        true
    }

    fn is(&self, key: &str, x: i32, y: i32) -> bool {
        x >= 0
            && y >= 0
            && self
                .at(x as usize, y as usize)
                .is_some_and(|c| c.encode_utf8(&mut [0; 4]) == key)
    }

    fn size(&self) -> (i32, i32) {
        let w = self.rows.iter().map(|r| r.chars().count()).max();
        (w.unwrap_or(0) as i32, self.rows.len() as i32)
    }
}

impl Tilemap {
    pub fn parse(text: &str) -> Result<Tilemap, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    pub fn layer(&self, name: &str) -> Option<&TileLayer> {
        self.layers.iter().find(|l| l.name == name)
    }

    pub fn tile(&self, layer: &TileLayer, x: usize, y: usize) -> Option<&TileDef> {
        let c = layer.at(x, y).filter(|c| *c != EMPTY)?;
        self.tiles.get(c.encode_utf8(&mut [0; 4]) as &str)
    }

    pub fn collision(&self, x: usize, y: usize) -> TileCollision {
        self.layers
            .iter()
            .rev()
            .filter_map(|l| self.tile(l, x, y))
            .map(|t| t.collision)
            .find(|c| !matches!(c, TileCollision::Behind | TileCollision::Ignore))
            .unwrap_or(self.empty)
    }

    pub fn is_blocked(&self, x: usize, y: usize) -> bool {
        self.collision(x, y) == TileCollision::Block
    }

    pub fn position(&self, x: f32, y: f32, elevation: f32) -> Vec3 {
        let a = self.origin[0] + x * self.cell;
        let b = self.origin[1] + y * self.cell;
        match self.plane {
            TilePlane::Xy => Vec3::new(a, b, 0.0),
            TilePlane::Xz => Vec3::new(a, elevation, b),
        }
    }

    pub fn placements(&self) -> Vec<TilePlacement> {
        let mut out = Vec::new();
        for layer in &self.layers {
            self.place_layer(layer, &mut out);
        }
        out.extend(self.objects.iter().map(|o| TilePlacement {
            pos: self.position(o.pos[0], o.pos[1], o.elevation),
            sprite: o.sprite.clone(),
            prefab: o.prefab.clone(),
            layer: o.order,
        }));
        out
    }

    fn place_layer(&self, layer: &TileLayer, out: &mut Vec<TilePlacement>) {
        let (w, h) = layer.size();
        for (key, tile) in &self.tiles {
            let on = |x: i32, y: i32| layer.is(key, x, y);
            let at = |x: f32, y: f32| self.position(x, y, layer.elevation);
            if tile.auto == AutoTile::Dual {
                for (x, y, index) in dual_cells((0, w - 1), (0, h - 1), on) {
                    if tile.skip_full && index == DUAL_FULL {
                        continue;
                    }
                    let mut placed = tile.placed(at(x, y), Some(index.to_string()), layer.order);
                    if let Some(sprite) = tile.seam_at(layer, x, y) {
                        placed.sprite = Some(sprite);
                    }
                    out.push(placed);
                }
            }
            for y in 0..h {
                for x in (0..w).filter(|&x| on(x, y)) {
                    let pos = at(x as f32, y as f32);
                    if let Some(fill) = &tile.fill {
                        out.push(TilePlacement {
                            pos,
                            sprite: Some(fill.clone()),
                            prefab: None,
                            layer: tile.fill_layer + layer.order,
                        });
                    }
                    let suffix = match tile.auto {
                        AutoTile::Dual => continue,
                        AutoTile::None => None,
                        AutoTile::Neighbors4 => Some(neighbor_index(x, y, on).to_string()),
                        AutoTile::Neighbors8 => Some(neighbor8_index(x, y, on).to_string()),
                        AutoTile::Nine => {
                            let (r, c) = nine_index(x, y, on);
                            Some(format!("{r}_{c}"))
                        }
                    };
                    out.push(tile.placed(pos, suffix, layer.order));
                }
            }
        }
    }
}

impl TileDef {
    fn seam_at(&self, layer: &TileLayer, x: f32, y: f32) -> Option<String> {
        let (seam, sprite) = (self.seam.as_deref()?, self.seam_sprite.as_deref()?);
        let (vx, vy) = ((x + 0.5) as i32, (y + 0.5) as i32);
        let mask = corner_index(vx, vy, |a, b| layer.is(seam, a, b));
        (mask != 0).then(|| format!("{sprite}_{mask}"))
    }

    fn placed(&self, pos: Vec3, suffix: Option<String>, order: i32) -> TilePlacement {
        let name = |base: &String| match &suffix {
            Some(s) => format!("{base}_{s}"),
            None => base.clone(),
        };
        TilePlacement {
            pos,
            sprite: self.sprite.as_ref().map(name),
            prefab: self.prefab.as_ref().map(name),
            layer: self.layer
                + order
                + match self.collision {
                    TileCollision::Behind => BEHIND_ORDER,
                    _ => 0,
                },
        }
    }
}

#[cfg(feature = "scene")]
fn tile_object(world: &World, map: &Tilemap, tile: &TilePlacement) -> GameObjDef {
    let prefab = tile.prefab.as_deref().and_then(|key| {
        let id = world.resource::<Prefabs>().ok()?.id_of(key);
        if id.is_none() {
            log::warn!("tilemap: missing prefab `{key}`");
        }
        id
    });
    let obj = GameObjDef::at(prefab, tile.pos, Vec3::ZERO, Vec3::ONE);
    #[cfg(feature = "2d")]
    let obj = with_sprite(obj, map, tile);
    #[cfg(not(feature = "2d"))]
    let _ = map;
    obj
}

#[cfg(all(feature = "2d", feature = "scene"))]
fn with_sprite(mut obj: GameObjDef, map: &Tilemap, tile: &TilePlacement) -> GameObjDef {
    if let Some(sprite) = &tile.sprite {
        let def = crate::scene::d2::SpriteDef {
            sprite: sprite.clone(),
            size: [map.cell, map.cell],
            layer: tile.layer,
            ..Default::default()
        };
        obj.components
            .push(crate::scene::def::make_component("sprite", &def));
    }
    obj
}

#[cfg(feature = "scene")]
pub fn spawn_tilemap(world: &mut World, map: &Tilemap) -> Vec<Entity> {
    let objects: Vec<GameObjDef> = map
        .placements()
        .iter()
        .map(|tile| tile_object(world, map, tile))
        .collect();
    objects
        .iter()
        .filter_map(|o| spawn_object(world, o))
        .collect()
}

#[cfg(feature = "scene")]
pub fn register(reg: &mut SpawnRegistry) {
    reg.categorize("Scene", &["tilemap"]);
    reg.add_with_defaults(
        "tilemap",
        |world, v, e| match v.clone().try_into::<Tilemap>() {
            Ok(map) => {
                for child in spawn_tilemap(world, &map) {
                    set_parent(world, child, e);
                }
            }
            Err(err) => log::error!("tilemap: {err}"),
        },
        crate::scene::defaults_of::<Tilemap>,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAP: &str = r##"
origin = [10.0, 20.0]
cell = 2.0

[[layer]]
name = "floor"
rows = ["#.", ".T"]

[[layer]]
name = "roof"
order = 5
elevation = 3.0
rows = ["..", ".R"]

[[object]]
pos = [0.5, 1.5]
sprite = "lamp"
order = 7

[tiles."#"]
sprite = "wall"
collision = "block"

[tiles.T]
prefab = "tree"

[tiles.R]
sprite = "roof"
"##;

    fn layer(rows: &[&str]) -> Vec<TileLayer> {
        vec![TileLayer {
            rows: rows.iter().map(|r| r.to_string()).collect(),
            ..Default::default()
        }]
    }

    #[test]
    fn reads_layers_objects_and_positions() {
        let m = Tilemap::parse(MAP).unwrap();
        assert!(m.is_blocked(0, 0));
        assert!(!m.is_blocked(1, 0));
        assert_eq!(m.placements().len(), 4);
        assert_eq!(m.position(1.0, 1.0, 0.0), Vec3::new(12.0, 22.0, 0.0));
        let roof = m
            .placements()
            .into_iter()
            .find(|p| p.sprite.as_deref() == Some("roof"));
        assert_eq!(roof.map(|r| r.layer), Some(5));
        let lamp = m.placements().pop().unwrap();
        assert_eq!(lamp.pos, Vec3::new(11.0, 23.0, 0.0));
        assert_eq!(lamp.layer, 7);
    }

    fn collision_map(rows: &[&[&str]]) -> Tilemap {
        let tile = |collision| TileDef {
            sprite: Some("t".into()),
            collision,
            ..Default::default()
        };
        Tilemap {
            layers: rows.iter().flat_map(|r| layer(r)).collect(),
            tiles: BTreeMap::from([
                ("b".to_string(), tile(TileCollision::Block)),
                ("p".to_string(), tile(TileCollision::Pass)),
                ("s".to_string(), tile(TileCollision::Behind)),
            ]),
            ..Default::default()
        }
    }

    #[test]
    fn top_layer_decides_and_behind_defers_to_the_layer_below() {
        assert!(!collision_map(&[&["b"], &["p"]]).is_blocked(0, 0));
        assert!(collision_map(&[&["p"], &["b"]]).is_blocked(0, 0));
        assert!(collision_map(&[&["b"], &["s"]]).is_blocked(0, 0));
        assert!(!collision_map(&[&["p"], &["s"]]).is_blocked(0, 0));
        assert!(!collision_map(&[&["s"]]).is_blocked(0, 0));
    }

    #[test]
    fn ignored_tiles_defer_and_empty_cells_use_the_map_default() {
        let mut m = collision_map(&[&["b."], &["i."]]);
        m.tiles.insert(
            "i".into(),
            TileDef {
                collision: TileCollision::Ignore,
                ..Default::default()
            },
        );
        assert!(m.is_blocked(0, 0));
        assert!(!m.is_blocked(1, 0));
        m.empty = TileCollision::Block;
        assert!(m.is_blocked(1, 0));
    }

    #[test]
    fn dual_pieces_touching_a_seam_use_the_seam_sprite() {
        let m = Tilemap {
            layers: layer(&["RRR", ".D."]),
            tiles: BTreeMap::from([(
                "R".to_string(),
                TileDef {
                    sprite: Some("roof".into()),
                    auto: AutoTile::Dual,
                    seam: Some("D".into()),
                    seam_sprite: Some("door".into()),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let names: Vec<_> = m
            .placements()
            .into_iter()
            .filter_map(|p| p.sprite)
            .collect();
        assert!(names.iter().any(|n| n == "door_8"));
        assert!(names.iter().any(|n| n == "door_4"));
    }

    #[test]
    fn behind_tiles_draw_over_the_character() {
        let p = collision_map(&[&["s"]]).placements();
        assert_eq!(p[0].layer, BEHIND_ORDER);
    }

    #[test]
    fn xz_plane_uses_elevation_as_height() {
        let mut m = Tilemap::parse(MAP).unwrap();
        m.plane = TilePlane::Xz;
        assert_eq!(m.position(1.0, 1.0, 3.0), Vec3::new(12.0, 3.0, 22.0));
    }

    #[test]
    fn neighbors_pick_the_indexed_tile() {
        for (auto, a, b) in [
            (AutoTile::Neighbors4, "wall_2", "wall_8"),
            (AutoTile::Neighbors8, "wall_2", "wall_8"),
        ] {
            let m = Tilemap {
                layers: layer(&["##"]),
                tiles: BTreeMap::from([(
                    "#".to_string(),
                    TileDef {
                        sprite: Some("wall".into()),
                        auto,
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            };
            let names: Vec<_> = m
                .placements()
                .into_iter()
                .filter_map(|p| p.sprite)
                .collect();
            assert_eq!(names, [a, b]);
        }
    }

    #[test]
    fn dual_places_tiles_on_the_offset_grid_for_prefabs_too() {
        let m = Tilemap {
            plane: TilePlane::Xz,
            layers: layer(&["#"]),
            tiles: BTreeMap::from([(
                "#".to_string(),
                TileDef {
                    prefab: Some("rock".into()),
                    auto: AutoTile::Dual,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let p = m.placements();
        assert_eq!(p.len(), 4);
        assert_eq!(p[0].pos, Vec3::new(-0.5, 0.0, -0.5));
        assert_eq!(p[0].prefab.as_deref(), Some("rock_8"));
    }

    #[cfg(feature = "scene")]
    #[test]
    fn tiles_spawn_from_prefabs() {
        let dir = std::env::temp_dir().join("tungstengine_tilemap_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("tree.toml"),
            "[[component]]\ntype = \"transform\"\nposition = [0.0, 0.0, 0.0]\n",
        )
        .unwrap();
        let mut world = crate::ecs::World::new();
        let mut reg = SpawnRegistry::default();
        crate::scene::register_builtin(&mut reg);
        world.insert_resource(reg);
        world.insert_resource(crate::scene::load_prefabs(&dir));
        let map = Tilemap {
            layers: layer(&["T"]),
            tiles: BTreeMap::from([(
                "T".to_string(),
                TileDef {
                    prefab: Some("tree".into()),
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        assert_eq!(spawn_tilemap(&mut world, &map).len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
