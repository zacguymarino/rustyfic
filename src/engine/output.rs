use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum OutputBlock {
    Title(String),
    Text(String),
    Event(String),
    Exits(String),
}

#[derive(Default, Debug, Serialize)]
pub struct Output {
    pub blocks: Vec<OutputBlock>,
}

impl Output {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(&mut self, s: impl Into<String>) {
        let s = s.into();
        if s.trim().is_empty() {
            return;
        }

        if let Some(pos) = self
            .blocks
            .iter()
            .position(|b| matches!(b, OutputBlock::Exits(_)))
        {
            self.blocks.insert(pos, OutputBlock::Title(s));
        } else {
            self.blocks.push(OutputBlock::Title(s));
        }
    }

    pub fn say(&mut self, s: impl Into<String>) {
        let s = s.into();
        if s.trim().is_empty() {
            return;
        }

        if let Some(pos) = self
            .blocks
            .iter()
            .position(|b| matches!(b, OutputBlock::Exits(_)))
        {
            self.blocks.insert(pos, OutputBlock::Text(s));
        } else {
            self.blocks.push(OutputBlock::Text(s));
        }
    }

    pub fn event(&mut self, s: impl Into<String>) {
        let s = s.into();
        if s.trim().is_empty() {
            return;
        }

        // If Exits is already present, keep it last by inserting before it.
        if let Some(pos) = self
            .blocks
            .iter()
            .position(|b| matches!(b, OutputBlock::Exits(_)))
        {
            self.blocks.insert(pos, OutputBlock::Event(s));
        } else {
            self.blocks.push(OutputBlock::Event(s));
        }
    }

    pub fn set_exits(&mut self, s: impl Into<String>) {
        let s = s.into();
        if s.trim().is_empty() {
            return;
        }

        // ensure only one Exits block exists, always last
        self.blocks.retain(|b| !matches!(b, OutputBlock::Exits(_)));
        self.blocks.push(OutputBlock::Exits(s));
    }
}
