//! Раздел 4.6: арена и индексы вместо ссылок.

use alloc::vec;
use alloc::vec::Vec;

// ANCHOR: arena
/// Номер узла в арене: 4 байта вместо 8-байтового указателя на x86_64.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(u32);

/// Устройство на шине. Родитель и дети — номера, а не ссылки, поэтому
/// узлы ссылаются друг на друга в обе стороны.
pub struct Node {
    pub name: &'static str,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

/// Все узлы лежат в одном векторе и принадлежат дереву.
pub struct Tree {
    nodes: Vec<Node>,
}

impl Tree {
    pub fn new(root: &'static str) -> (Tree, NodeId) {
        let root = Node { name: root, parent: None, children: Vec::new() };
        (Tree { nodes: vec![root] }, NodeId(0))
    }

    pub fn add(&mut self, parent: NodeId, name: &'static str) -> NodeId {
        let id = NodeId(u32::try_from(self.nodes.len()).expect("больше 2^32 узлов"));
        self.nodes.push(Node { name, parent: Some(parent), children: Vec::new() });
        self.nodes[parent.0 as usize].children.push(id);
        id
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id.0 as usize]
    }

    /// Путь от корня до узла: вверх по родителям.
    pub fn path(&self, mut id: NodeId) -> Vec<&'static str> {
        let mut path = vec![self.node(id).name];
        while let Some(p) = self.node(id).parent {
            path.push(self.node(p).name);
            id = p;
        }
        path.reverse();
        path
    }

    /// Переименовать узел: изменяемый доступ к одному узлу не мешает
    /// держать номера других.
    pub fn rename(&mut self, id: NodeId, name: &'static str) {
        self.nodes[id.0 as usize].name = name;
    }
}
// ANCHOR_END: arena

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree() {
        let (mut t, root) = Tree::new("mcu");
        let i2c = t.add(root, "i2c1");
        let spi = t.add(root, "spi1");
        let bme = t.add(i2c, "bme280");
        let flash = t.add(spi, "w25q");
        assert_eq!(t.path(bme), ["mcu", "i2c1", "bme280"]);
        assert_eq!(t.node(root).children, [i2c, spi]);
        t.rename(flash, "w25q128");
        assert_eq!(t.path(flash), ["mcu", "spi1", "w25q128"]);
        assert_eq!(size_of::<NodeId>(), 4);
        // Без ниши `Option<NodeId>` вдвое больше самого номера.
        assert_eq!(size_of::<Option<NodeId>>(), 8);
        // С нишей — столько же, сколько сам номер (раздел 6.4).
        struct NzId(#[allow(dead_code)] core::num::NonZeroU32);
        assert_eq!(size_of::<Option<NzId>>(), 4);
    }
}
