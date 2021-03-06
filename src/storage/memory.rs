use storage::{StorageEngine, BlockStorage, Result};
use blocks::BlockPointer;

use std::collections::BTreeMap;

#[derive(Default)]
pub struct MemoryStorage {
    blocks: BTreeMap<BlockPointer, Vec<u8>>,
    head: BlockPointer,
}

impl MemoryStorage {
    pub fn new() -> MemoryStorage {
        MemoryStorage::default()
    }

    #[inline]
    pub fn into_engine(self) -> StorageEngine {
        StorageEngine::Memory(self)
    }
}

impl BlockStorage for MemoryStorage {
    fn write_bytes(&mut self, pointer: &BlockPointer, bytes: Vec<u8>) -> Result<()> {
        self.blocks.insert(pointer.clone(), bytes);
        Ok(())
    }

    fn get_bytes(&self, pointer: &BlockPointer) -> Result<Vec<u8>> {
        match self.blocks.get(pointer) {
            Some(bytes) => Ok(bytes.clone()),
            None => panic!("not found"), // TODO
        }
    }

    fn get_head(&self) -> Result<BlockPointer> {
        Ok(self.head.clone())
    }

    fn update_head(&mut self, pointer: &BlockPointer) -> Result<()> {
        self.head = pointer.clone();
        Ok(())
    }
}
