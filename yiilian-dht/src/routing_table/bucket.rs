
use yiilian_core::common::error::Error;

use crate::common::id::Id;

use super::Node;

#[derive(Debug)]
pub struct Buckets {
    local_id: Id,
    buckets: Vec<Vec<Node>>,
    k: usize,
}

impl Buckets {
    pub fn new(k: usize, local_id: Id) -> Buckets {
        let mut to_ret = Buckets {
            local_id,
            buckets: Vec::with_capacity(32),
            k,
        };

        to_ret.buckets.push(Vec::new());

        to_ret
    }

    pub fn set_id(&mut self, new_id: Id) {
        self.clear();
        self.local_id = new_id;
    }

    pub fn count(&self) -> usize {
        let mut count = 0;
        for bucket in &self.buckets {
            count += bucket.len();
        }

        count
    }

    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&Node) -> bool,
    {
        for bucket in &mut self.buckets {
            bucket.retain(|item| f(item));
        }
    }

    pub fn get_node_mut(&mut self, id: &Id) -> Option<&mut Node> {
        match self.get_dest_bucket_idx_for_id(id) {
            Ok(dest_bucket_idx) => {
                if let Some(bucket) = self.buckets.get_mut(dest_bucket_idx) {
                    for item in bucket.iter_mut() {
                        if item.id == *id {
                            return Some(item);
                        }
                    }
                }
                None
            },
            Err(_) => None,
        }
    }

    /// 根据节点ID选择一个其适合的 kbuckets 的索引(就是找出适合存放该node_id的 kbucket，在 kbuckets 列表中的 index)
    /// 根据目标 ID 和 local ID 来获得 cpl，取 cpl 或 buckets 列表长度的最小值，作为 node_id 的 bucket idx
    fn get_dest_bucket_idx_for_id(&self, id: &Id) -> Result<usize, Error> {
        // 根据目标 ID 和 local ID 来获得 cpl
        let cpl = self.local_id.matching_prefix_bits(id);
        let rst = std::cmp::min(self.buckets.len() - 1, cpl);

        Ok(rst)
    }

    /// 根据节点 item.ID 选择一个其适合的 kbuckets 的索引(就是找出适合存放该node_id的 kbucket，在 kbuckets 列表中的 index)
    fn get_dest_bucket_idx(&self, item: &Node) -> Result<usize, Error> {
        self.get_dest_bucket_idx_for_id(&item.id)
    }

    pub fn add(&mut self, item: Node, chump_list: Option<&mut Vec<Node>>) -> Result<(), Error> {
        // Never add our own node!
        if item.id == self.local_id {
            return Ok(());
        }

        // 获取目标 bucket 索引
        let dest_bucket_idx = self.get_dest_bucket_idx(&item)?;
        self.buckets[dest_bucket_idx].push(item);
        // 处理 buckets[bucket_index] 的溢出，k 桶分裂
        self.handle_bucket_overflow(dest_bucket_idx, chump_list)?;

        Ok(())
    }

    pub fn clear(&mut self) {
        self.buckets.clear();
        self.buckets.push(Vec::with_capacity(2 * self.k));
    }

    /// 处理 buckets[bucket_index] 及其之后的每个 bucket 的溢出
    fn handle_bucket_overflow(
        &mut self,
        mut bucket_index: usize,
        mut chump_list: Option<&mut Vec<Node>>,
    ) -> Result<(), Error> {
        while bucket_index < self.buckets.len() {
            // Is the bucket over capacity?
            if self.buckets[bucket_index].len() > self.k {
                // Is this the "deepest" bucket?
                // If so, add a new one since we're over capacity
                // 如果是最后一个 kbucket，溢出后则在 buckets 里增加一个新 bucket
                if bucket_index == self.buckets.len() - 1 {
                    self.buckets.push(Vec::with_capacity(2 * self.k));
                }

                // 注意：由于 local id 可能被更新，所以 cpl 会被重新计算，
                //      因此后面的处理没包含在上面的 if bucket_index == self.buckets.len() - 1 的条件处理中
                //      这样当 local id 改变时，调用该该函数，并传递 bucket_index = 0, 则将重新整理所有的 bucket
                // (Hopefully) move some nodes out of this bucket into the next one
                for i in (0..self.buckets[bucket_index].len()).rev() { // 后进先出
                    let ideal_bucket_idx = self.get_dest_bucket_idx(&self.buckets[bucket_index][i])?;

                    // This Node belongs in another bucket. Move it.
                    if ideal_bucket_idx != bucket_index {
                        let node = self.buckets[bucket_index].swap_remove(i);
                        self.buckets[ideal_bucket_idx].push(node); // 加入后面的 bucket 中去了，所以要用 while 循环处理后面的 bucket
                    }
                }

                // Sort by oldest. Move the newest excess to the chump list
                // 如果经过上面溢出处理，buckets[bucket_index] 的长度还是超过 k，
                // 则将 buckets[bucket_index] 中新加入的 node 移除
                if self.buckets[bucket_index].len() > self.k {
                    // 根据 first_seen 时间升序排列
                    self.buckets[bucket_index].sort_unstable_by_key(|a| a.get_first_seen()); 

                    // 在 K 处分裂，remainder 中是被移除的新加入的 node 
                    let mut remainder = self.buckets[bucket_index].split_off(self.k); 

                    if let Some(chump_list) = &mut chump_list {
                        chump_list.append(&mut remainder);
                    }
                }
            }

            // 继续处理下一个 K 桶的溢出
            bucket_index += 1
        }

        Ok(())
    }

    pub fn remove(&mut self, id: &Id) -> Option<Node> {
        match self.get_dest_bucket_idx_for_id(id) {
            Ok(dest_bucket_idx) => {
                if let Some(bucket) = self.buckets.get_mut(dest_bucket_idx) {
                    for i in 0..bucket.len() {
                        let node =  &bucket[i];
                        if node.id == *id {
                            log::trace!(target:"yiilian_dht::bucket", "remove node: id = {}, addr = {:?}",  node.id, node.address);
                            return Some(bucket.swap_remove(i));
                        }
                    }
                }
                None
            },
            Err(_) => None,
        }
    }

    /// 获取 buckets 列表中所有 bucket 中的 node
    pub fn values(&self) -> Vec<&Node> {
        let mut to_ret = Vec::new();
        for bucket in &self.buckets {
            for item in bucket {
                to_ret.push(item);
            }
        }
        to_ret
    }

    pub fn contains(&self, id: &Id) -> bool {
        let dest_bucket_idx = self.get_dest_bucket_idx_for_id(id);
        if let Ok(dest_bucket_idx) = dest_bucket_idx {
            match self.buckets.get(dest_bucket_idx) {
                Some(bucket) => {
                    for item in bucket.iter() {
                        if item.id == *id {
                            return true;
                        }
                    }
                },
                None => return false,
            }
        }

        false
    }

    /// 返回 K 个邻居节点，根据目标节点到本机节点的距离，升序排列（由近及远）
    pub fn get_nearest_nodes(&self, id: &Id, exclude: Option<&Id>) -> Vec<&Node> {
        let mut all: Vec<&Node> = self
            .values()
            .iter()
            .filter(|item| exclude.is_none() || *exclude.unwrap() != item.id)
            .copied()
            .collect();

        // 对所有节点，根据目标节点到本机节点的距离，升序排列（由近及远）
        all.sort_unstable_by(|a, b| {
            let a_dist = a.id.xor(id);
            let b_dist = b.id.xor(id);
            a_dist.partial_cmp(&b_dist).unwrap()
        });

        // 获取其中 K 及距离 id 最近的节点
        all.truncate(self.k);
        
        all
    }

    #[allow(unused)]
    pub fn len(&self) -> usize {
        let mut n = 0;
        for bucket in &self.buckets {
            n += bucket.len();
        }
        
        n
    }
}