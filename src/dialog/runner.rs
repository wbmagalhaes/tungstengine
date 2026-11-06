use std::any::Any;
use std::collections::HashMap;
use std::path::Path;

use yarnspinner::prelude::*;
use yarnspinner::runtime::{MemoryVariableStorage, TextProvider};

pub enum Presentation {
    Line { actor: Option<String>, text: String },
    Options(Vec<String>),
    Done,
}

#[derive(Clone, Debug, Default)]
struct LocalizedText {
    base: HashMap<LineId, String>,
    translations: HashMap<Language, HashMap<LineId, String>>,
    language: Option<Language>,
}

impl TextProvider for LocalizedText {
    fn clone_shallow(&self) -> Box<dyn TextProvider> {
        Box::new(self.clone())
    }

    fn accept_line_hints(&mut self, _line_ids: &[LineId]) {}

    fn get_text(&self, id: &LineId) -> Option<String> {
        let translated = self
            .language
            .as_ref()
            .and_then(|language| self.translations.get(language))
            .and_then(|table| table.get(id));
        translated.or_else(|| self.base.get(id)).cloned()
    }

    fn set_language(&mut self, language: Option<Language>) {
        self.language = language;
    }

    fn get_language(&self) -> Option<Language> {
        self.language.clone()
    }

    fn are_lines_available(&self) -> bool {
        !self.base.is_empty()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct YarnRunner {
    dialogue: Dialogue,
    options: Vec<OptionId>,
    commands: Vec<String>,
}

impl YarnRunner {
    pub fn from_dir(dir: &Path) -> Option<Self> {
        let mut compiler = YarnCompiler::new();
        let mut any = false;
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yarn") {
                compiler.read_file(&path);
                any = true;
            }
        }
        if !any {
            return None;
        }
        Self::build(compiler)
    }

    pub fn from_source(name: &str, source: &str) -> Option<Self> {
        let mut compiler = YarnCompiler::new();
        compiler.add_file(YarnFile {
            file_name: name.to_string(),
            source: source.to_string(),
        });
        Self::build(compiler)
    }

    fn build(compiler: YarnCompiler) -> Option<Self> {
        let compilation = compiler
            .compile()
            .map_err(|e| log::error!("yarn compile: {e}"))
            .ok()?;
        let program = compilation.program?;
        let text = LocalizedText {
            base: compilation
                .string_table
                .into_iter()
                .map(|(id, info)| (id, info.text))
                .collect(),
            ..LocalizedText::default()
        };
        let mut dialogue = Dialogue::new(Box::new(MemoryVariableStorage::new()), Box::new(text));
        dialogue.add_program(program);
        Some(YarnRunner {
            dialogue,
            options: Vec::new(),
            commands: Vec::new(),
        })
    }

    pub fn add_translation(&mut self, locale: &str, lines: HashMap<String, String>) {
        let provider = self.dialogue.text_provider_mut().as_any_mut();
        let Some(text) = provider.downcast_mut::<LocalizedText>() else {
            return;
        };
        let table = text.translations.entry(Language::from(locale)).or_default();
        table.extend(
            lines
                .into_iter()
                .map(|(id, line)| (LineId(format!("line:{id}")), line)),
        );
    }

    pub fn set_language(&mut self, locale: &str) {
        self.dialogue.set_language_code(Language::from(locale));
    }

    pub fn set_flags(&mut self, flags: &[String]) {
        let storage = self.dialogue.variable_storage_mut();
        for f in flags {
            let _ = storage.set(yarn_var(f), true.into());
        }
    }

    pub fn start(&mut self, node: &str) -> bool {
        self.options.clear();
        self.dialogue.set_node(node).is_ok()
    }

    pub fn advance(&mut self) -> Presentation {
        loop {
            let events = match self.dialogue.continue_() {
                Ok(events) => events,
                Err(e) => {
                    log::error!("yarn run: {e}");
                    return Presentation::Done;
                }
            };
            for event in events {
                match event {
                    DialogueEvent::Line(line) => {
                        return Presentation::Line {
                            actor: line.character_name().map(str::to_string),
                            text: marked_text(&line),
                        };
                    }
                    DialogueEvent::Options(opts) => {
                        self.options = opts.iter().map(|o| o.id).collect();
                        return Presentation::Options(
                            opts.iter().map(|o| line_text(&o.line)).collect(),
                        );
                    }
                    DialogueEvent::Command(cmd) => self.commands.push(cmd.raw),
                    DialogueEvent::DialogueComplete => return Presentation::Done,
                    _ => {}
                }
            }

            if !self.dialogue.can_continue() {
                return Presentation::Done;
            }
        }
    }

    pub fn drain_commands(&mut self) -> Vec<String> {
        std::mem::take(&mut self.commands)
    }

    pub fn choose(&mut self, index: usize) {
        if let Some(id) = self.options.get(index).copied() {
            let _ = self.dialogue.set_selected_option(id);
        }
        self.options.clear();
    }
}

fn yarn_var(flag: &str) -> String {
    let mut s = String::from("$");
    for c in flag.chars() {
        s.push(if c.is_ascii_alphanumeric() { c } else { '_' });
    }
    s
}

fn line_text(line: &YarnLine) -> String {
    if line.character_name().is_some() {
        line.text
            .split_once(':')
            .map(|(_, rest)| rest.trim().to_string())
            .unwrap_or_else(|| line.text.clone())
    } else {
        line.text.clone()
    }
}

fn value_text(value: &MarkupValue) -> String {
    match value {
        MarkupValue::Integer(v) => v.to_string(),
        MarkupValue::Float(v) => v.to_string(),
        MarkupValue::String(v) => v.clone(),
        MarkupValue::Bool(v) => v.to_string(),
    }
}

fn marked_text(line: &YarnLine) -> String {
    let chars: Vec<char> = line.text.chars().collect();
    let mut start = 0;
    let mut opens: Vec<(usize, String)> = Vec::new();
    let mut closes: Vec<(usize, String)> = Vec::new();
    for attr in &line.attributes {
        if attr.name == "character" {
            start = attr.position + attr.length;
            continue;
        }
        let value = attr.properties.get(&attr.name).map(value_text);
        let tag = match value {
            Some(value) => format!("[{}={value}]", attr.name),
            None => format!("[{}]", attr.name),
        };
        opens.push((attr.position, tag));
        if attr.length > 0 {
            closes.push((attr.position + attr.length, format!("[/{}]", attr.name)));
        }
    }
    let mut out = String::new();
    for i in start..=chars.len() {
        for (_, tag) in closes.iter().filter(|(at, _)| *at == i) {
            out.push_str(tag);
        }
        for (_, tag) in opens.iter().filter(|(at, _)| *at == i) {
            out.push_str(tag);
        }
        if let Some(ch) = chars.get(i) {
            out.push(*ch);
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "title: t\n---\nguardiao: Ola\n-> A\n    guardiao: escolheu A\n-> B\n    guardiao: escolheu B\n===\n";

    #[test]
    fn surfaces_commands() {
        let src = "title: t\n---\nguardiao: Ola\n<<reveal act:guardiao>>\nguardiao: De novo\n===\n";
        let mut r = YarnRunner::from_source("t.yarn", src).expect("compiled");
        assert!(r.start("t"));
        assert!(matches!(r.advance(), Presentation::Line { .. }));
        assert!(r.drain_commands().is_empty());
        assert!(matches!(r.advance(), Presentation::Line { .. }));
        assert_eq!(r.drain_commands(), vec!["reveal act:guardiao"]);
    }

    const BRANCH: &str = "title: t\n---\n<<declare $door_hall = false>>\n<<if $door_hall>>\nplayer: aberta\n<<else>>\nplayer: trancada\n<<endif>>\n===\n";

    #[test]
    fn flag_drives_branch() {
        let mut locked = YarnRunner::from_source("t.yarn", BRANCH).expect("compiled");
        assert!(locked.start("t"));
        match locked.advance() {
            Presentation::Line { text, .. } => assert_eq!(text, "trancada"),
            _ => panic!("expected line"),
        }

        let mut open = YarnRunner::from_source("t.yarn", BRANCH).expect("compiled");
        open.set_flags(&["door:hall".to_string()]);
        assert!(open.start("t"));
        match open.advance() {
            Presentation::Line { text, .. } => assert_eq!(text, "aberta"),
            _ => panic!("expected line"),
        }
    }

    #[test]
    fn flag_command_surfaces() {
        let src = "title: t\n---\nplayer: clic\n<<flag door:hall>>\n===\n";
        let mut r = YarnRunner::from_source("t.yarn", src).expect("compiled");
        assert!(r.start("t"));
        assert!(matches!(r.advance(), Presentation::Line { .. }));
        assert!(matches!(r.advance(), Presentation::Done));
        assert_eq!(r.drain_commands(), vec!["flag door:hall"]);
    }

    #[test]
    fn runs_a_node() {
        let mut r = YarnRunner::from_source("t.yarn", SRC).expect("compiled");
        assert!(r.start("t"));
        match r.advance() {
            Presentation::Line { actor, text } => {
                assert_eq!(actor.as_deref(), Some("guardiao"));
                assert_eq!(text, "Ola");
            }
            _ => panic!("expected line"),
        }
        match r.advance() {
            Presentation::Options(opts) => assert_eq!(opts, vec!["A", "B"]),
            _ => panic!("expected options"),
        }
        r.choose(0);
        match r.advance() {
            Presentation::Line { text, .. } => assert_eq!(text, "escolheu A"),
            _ => panic!("expected line"),
        }
        assert!(matches!(r.advance(), Presentation::Done));
    }
}
