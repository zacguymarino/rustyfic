#[derive(Debug, Clone)]
pub enum OutputBlock {
    Title(String),
    Text(String),
    Event(String),
    Exits(String),
}

#[derive(Default, Debug)]
pub struct Output {
    pub blocks: Vec<OutputBlock>,
}

impl Output {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !s.trim().is_empty() {
            self.blocks.push(OutputBlock::Title(s));
        }
    }

    pub fn say(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !s.trim().is_empty() {
            self.blocks.push(OutputBlock::Text(s));
        }
    }

    pub fn event(&mut self, s: impl Into<String>) {
        let s = s.into();
        if !s.trim().is_empty() {
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
