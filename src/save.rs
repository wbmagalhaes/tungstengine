use crate::ecs::prelude::*;
use crate::storage;

pub const DEFAULT_SAVE_KEY: &str = "save.toml";

pub type SaveFn = fn(&World) -> toml::Value;
pub type LoadFn = fn(&mut World, &toml::Value);

#[derive(Resource)]
pub struct SaveRegistry {
    key: String,
    entries: Vec<(String, SaveFn, LoadFn)>,
}

impl Default for SaveRegistry {
    fn default() -> Self {
        SaveRegistry::new(DEFAULT_SAVE_KEY)
    }
}

impl SaveRegistry {
    pub fn new(key: &str) -> Self {
        SaveRegistry {
            key: key.to_string(),
            entries: Vec::new(),
        }
    }

    pub fn register(&mut self, key: &str, save: SaveFn, load: LoadFn) {
        self.entries.push((key.to_string(), save, load));
    }
}

pub fn save(world: &World) -> Result<(), String> {
    let reg = world
        .resource::<SaveRegistry>()
        .map_err(|_| "missing SaveRegistry".to_string())?;
    let mut table = toml::map::Map::new();
    for (key, save_fn, _) in &reg.entries {
        table.insert(key.clone(), save_fn(world));
    }
    let text = toml::to_string_pretty(&toml::Value::Table(table)).map_err(|e| e.to_string())?;
    storage::write(&reg.key, &text)
}

pub fn load(world: &mut World) {
    let Ok(reg) = world.resource::<SaveRegistry>() else {
        return;
    };
    let Some(table) = storage::read(&reg.key).and_then(|t| toml::from_str::<toml::Value>(&t).ok())
    else {
        return;
    };
    let loaders: Vec<(String, LoadFn)> = reg
        .entries
        .iter()
        .map(|(k, _, l)| (k.clone(), *l))
        .collect();
    for (key, load_fn) in loaders {
        if let Some(value) = table.get(&key) {
            load_fn(world, value);
        }
    }
}

pub mod prelude {
    pub use super::SaveRegistry;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct Score(i64);

    fn save_score(world: &World) -> toml::Value {
        toml::Value::Integer(world.resource::<Score>().map_or(0, |s| s.0))
    }

    fn load_score(world: &mut World, value: &toml::Value) {
        world.insert_resource(Score(value.as_integer().unwrap_or(0)));
    }

    #[test]
    fn round_trip_through_storage() {
        let path = std::env::temp_dir().join("tungstengine_save_test.toml");
        let mut reg = SaveRegistry::new(&path.to_string_lossy());
        reg.register("score", save_score, load_score);
        let mut world = World::new();
        world.insert_resource(reg);
        world.insert_resource(Score(7));
        save(&world).unwrap();
        world.insert_resource(Score(0));
        load(&mut world);
        assert_eq!(world.resource::<Score>().map(|s| s.0).unwrap(), 7);
        let _ = std::fs::remove_file(path);
    }
}
