use super::{get_block_cache, BlockDevice, BLOCK_SZ};
use alloc::sync::Arc;
/// A bitmap block
/// 将位图区域中的一个磁盘块解释为长度为 64 的一个 u64 数组
type BitmapBlock = [u64; 64];
/// Number of bits in a block
const BLOCK_BITS: usize = BLOCK_SZ * 8;
/// A bitmap
pub struct Bitmap {
    start_block_id: usize,
    blocks: usize,
}

/// Decompose bits into (block_pos, bits64_pos, inner_pos)
/// 将位分解为（block_pos、bits64_pos、inner_pos）
fn decomposition(mut bit: usize) -> (usize, usize, usize) {
    let block_pos = bit / BLOCK_BITS;
    bit %= BLOCK_BITS;
    (block_pos, bit / 64, bit % 64)
}

impl Bitmap {
    /// A new bitmap from start block id and number of blocks
    /// 根据起始块id和块数创建新位图
    pub fn new(start_block_id: usize, blocks: usize) -> Self {
        Self {
            start_block_id,
            blocks,
        }
    }
    /// Allocate a new block from a block device
    /// 从块设备分配新块 
    pub fn alloc(&self, block_device: &Arc<dyn BlockDevice>) -> Option<usize> {
        //枚举区域中的每个块（编号为 block_id ）
        for block_id in 0..self.blocks {
            let pos = get_block_cache(
                //调用 get_block_cache 获取块缓存
                block_id + self.start_block_id as usize,
                Arc::clone(block_device),
            )
            .lock()//通过 .lock() 获取块缓存的互斥锁从而可以对块缓存进行访问
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                //尝试在 bitmap_block 中找到一个空闲的bit并返回其位置
                if let Some((bits64_pos, inner_pos)) = bitmap_block
                    .iter()
                    .enumerate()
                    .find(|(_, bits64)| **bits64 != u64::MAX)
                    .map(|(bits64_pos, bits64)| (bits64_pos, bits64.trailing_ones() as usize))//返回分配的bit编号的时候
                {
                    // modify cache
                    bitmap_block[bits64_pos] |= 1u64 << inner_pos;
                    Some(block_id * BLOCK_BITS + bits64_pos * 64 + inner_pos as usize)
                } else {
                    None
                }
            });
            //我们一旦在某个块中找到一个空闲的bit并成功分配，就不再考虑后续的块。
            if pos.is_some() {
                return pos;
            }
        }
        None
    }
    /// Deallocate a block
    /// 释放一个块
    pub fn dealloc(&self, block_device: &Arc<dyn BlockDevice>, bit: usize) {
        let (block_pos, bits64_pos, inner_pos) = decomposition(bit);
        get_block_cache(block_pos + self.start_block_id, Arc::clone(block_device))
            .lock()
            .modify(0, |bitmap_block: &mut BitmapBlock| {
                assert!(bitmap_block[bits64_pos] & (1u64 << inner_pos) > 0);
                bitmap_block[bits64_pos] -= 1u64 << inner_pos;
            });
    }
    /// Get the max number of allocatable blocks
    /// 获取可分配块的最大数量
    pub fn maximum(&self) -> usize {
        self.blocks * BLOCK_BITS
    }
}
