pub struct Table {
    names: Vec<String>,
}

impl Table {
    /// Первое имя, начинающееся с `key`.
    pub fn lookup(&self, key: &str) -> Option<&String> {
        self.names.iter().find(|n| n.starts_with(key))
    }
}
