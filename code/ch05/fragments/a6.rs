pub struct Table {
    names: Vec<String>,
}

impl Table {
    /// Имя канала по номеру.
    pub fn name(&self, i: usize) -> &str {
        &self.names[i]
    }
}
