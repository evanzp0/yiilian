use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use chrono::Utc;
use log::trace;
use yiilian_core::{
    common::error::Error, except_result, net::block_list::BlockList
};

use crate::common::{context::dht_ctx_state, id::Id};

use super::{Buckets, Node};

#[derive(Debug)]
pub struct RoutingTable {
    ctx_index: u16,
    verified: Buckets,
    unverified: Buckets,

    /// IP block list
    pub block_list: BlockList,

    /// IP white list
    pub white_list: HashSet<IpAddr>,
}

impl RoutingTable {
    pub fn new(
        ctx_index: u16,
        k: usize,
        block_list: BlockList,
        local_id: Id,
    ) -> RoutingTable {

        block_list.prune_loop();

        RoutingTable {
            ctx_index,
            verified: Buckets::new(k, local_id),
            unverified: Buckets::new(k, local_id),
            block_list,
            white_list: HashSet::new(),
        }
    }

    /// 判断 addr 是否在黑名单中
    pub fn is_blocked(&self, addr: &SocketAddr) -> bool {
        self.block_list.contains(addr.ip(), addr.port())
    }

    /// 将一个节点加入 K 桶，如果节点已存在则更新 K 桶中该节点的状态
    ///
    /// # Parameters
    /// * `node` - 添加或更新的节点
    /// * `verified` - true，如果我们知道该节点在线且能通信，比如，我们发送一个请求，然后收到响应。
    /// 如果为 true，该节点的 `last_verified` 和 `last_seen` 属性将被更新
    /// 如果为 false， 只更新 `last_seen` 属性(我们没发送过请求，但收到过该节点的消息)
    pub fn add_or_update(
        &mut self,
        node: Node,
        verified: bool,
    ) -> Result<(), Error> {
        // // 入口节点不需要加入 routing_table
        // if self.white_list.contains(&node.address.ip()) {
        //     log::debug!(target: "yiilian_dht::routing_table", "Address: {} is in the white_list", node.address.ip());
        //     return Ok(())
        // }

        // 黑名单中的节点不需要加入 routing_table
        if self.is_blocked(&node.address) {
            Err(Error::new_block(&format!("{} is blocked", node.address)))?;
        }

        // 对方节点响应我方请求则 verified node
        if verified {
            self.add_or_update_verified(node)?;
        } else {
            // last_seen node 是指只要收到过该节点的消息
            self.add_or_update_last_seen(node)?;
        }

        except_result!(dht_ctx_state(self.ctx_index).write(), "Get writable state failed")
            .is_join_kad = self.verified.count() > 0;

        Ok(())
    }

    /// last_seen 是指最近我们收到过该节点的消息，不管我们是否发出过请求
    fn add_or_update_last_seen(&mut self, node: Node) -> Result<(), Error> {
        if let Some(existing) = self.verified.get_node_mut(&node.id) {
            trace!(target: "yiilian_dht::routing_table", "Updating existing verified node( id: {}, addr: {:?} ) last seen", node.id, node.address);
            existing.last_seen = Utc::now();
        } else if let Some(existing) = self.unverified.get_node_mut(&node.id) {
            trace!(target: "yiilian_dht::routing_table", "Updating existing unverified node( id: {}, addr: {:?} ) last seen", node.id, node.address);
            existing.last_seen = Utc::now();
        } else {
            trace!(target: "yiilian_dht::routing_table", "Attempting to add unverified node( id: {}, addr: {:?} )", node.id, node.address);
            self.unverified.add(node, None)?;
        }

        Ok(())
    }

    /// 将已验证节点加入 verified bucket，如果 verified bucket 已满，则加入 unverified bucket，如果还是满了，则抛弃
    fn add_or_update_verified(&mut self, mut node: Node) -> Result<(), Error> {
        let now = Utc::now();

        // Already exists in unverified.
        // Remove it and try to add it to Verified.
        // If verified is full, add whatever overflows back to unverified (if it fits)
        // 尝试将已验证节点从 Unverified 移除并加入 Verified，如果已满则加入 Unverified
        if let Some(mut item) = self.unverified.remove(&node.id) {
            trace!(target: "yiilian_dht::routing_table", "Attempting to move {:?} from unverified to verified", node);
            item.last_seen = now;
            item.last_verified = Some(now);
            let mut chump_list = Vec::with_capacity(1);
            self.verified.add(item, Some(&mut chump_list))?;

            for item in chump_list {
                self.unverified.add(item, None)?;
            }
        }
        // Already exists in verified.
        // Update it
        else if let Some(node) = self.verified.get_node_mut(&node.id) {
            trace!(target: "yiilian_dht::routing_table", "Marking verified node (id: {:?}, address: {:?}) as verified again", node.id, node.address);
            node.last_verified = Some(node.last_seen);
            node.last_seen = now;
            node.last_verified = Some(now);
        }
        // Doesn't exist yet
        else {
            trace!(target: "yiilian_dht::routing_table", "Marking new node (id: {:?}, address: {:?}) as verified", node.id, node.address);
            node.last_seen = now;
            node.last_verified = Some(now);

            let mut chump_list = Vec::with_capacity(1);
            self.verified.add(node, Some(&mut chump_list))?;

            for item in chump_list {
                self.unverified.add(item, None)?; // unverified 如果满了，则 node 被抛弃
            }
        }

        Ok(())
    }

    /// 返回所有 dht verfied 的节点
    pub fn get_all_verified(&self) -> Vec<Node> {
        self.verified.values().iter().copied().cloned().collect()
    }

    /// 返回所有 dht unverfied 的节点
    pub fn get_all_unverified(&self) -> Vec<Node> {
        self.unverified.values().iter().copied().cloned().collect()
    }

    /// 移除节点
    pub fn remove(&mut self, node_id: &Id) -> Option<Node> {

        let node_v = self.verified.remove(&node_id);
        let node_u = self.unverified.remove(&node_id);

        if node_v.is_some() {
            return node_v;
        } else if node_u.is_some() {
            return node_u;
        }

        except_result!(dht_ctx_state(self.ctx_index).write(), "Get writable state failed")
            .is_join_kad = self.verified.count() > 0;

        None
    }

    /// 加入阻塞列表，并从路由表中移除对应节点
    pub fn add_block_list(
        &mut self,
        dest_addr: SocketAddr,
        dest_id: Option<Id>,
        duration: Option<Duration>,
    ) {
        if self.white_list.contains(&dest_addr.ip()) {
            return;
        }

        self.block_list
            .insert(dest_addr.ip(), dest_addr.port().into(), duration);

        // 从路由表中删除该节点
        if let Some(dest_id) = dest_id {
            self.remove(&dest_id);
        }
    }

    /// 获取距离 id 更近的节点，结果中要排除掉 exclude 节点。
    pub fn get_nearest_nodes(&self, id: &Id, exclude: Option<&Id>) -> Vec<Node> {
        self.verified
            .get_nearest_nodes(id, exclude)
            .iter()
            .map(|node| (*node).clone())
            .collect()
    }

    pub fn count(&self) -> (usize, usize) {
        (self.unverified.count(), self.verified.count())
    }

    /// grace_period： 已验证节点的再次校验时间间隔，超过该时间间隔没有再次校验的节点将被删除
    /// unverified_grace_period：已验证节点的再校验时间间隔，超过该时间间隔没有再次校验的节点将被删除
    pub fn prune(
        &mut self,
        grace_period: Duration,
        unverified_grace_period: Duration,
    ) {
        let now = Utc::now();
        let time = now - grace_period;
        let unverified_time = now - unverified_grace_period;

        // 在 verified 中保留 当前时间 - grace_period = 截至时点，之后已被验证的节点
        self.verified.retain(|node| {
            if let Some(last_verified) = node.last_verified {
                return last_verified >= time;
            }
            // trace!(target: "yiilian_dht::RoutingTable", "Verified {:?} hasn't verified recently. Removing.", node);
            false
        });

        // 在 unverified 中保留 当前时间 - grace_period = 截至时点，之后已被验证或被 seen (我们没请求过，但收到过该节点的消息) 的节点
        self.unverified.retain(|node| {
            if let Some(last_verified) = node.last_verified {
                if last_verified >= time {
                    return true;
                }
            }
            if node.last_seen >= time && node.last_seen >= unverified_time {
                return true;
            }
            // trace!(target: "yiilian_dht::RoutingTable", "Unverified {:?} is dead. Removing", node);
            false
        });

        except_result!(dht_ctx_state(self.ctx_index).write(), "Get writable state failed")
            .is_join_kad = self.verified.count() > 0;
    }

    pub fn set_id(&mut self, new_id: Id) {
        self.verified.set_id(new_id);
        self.unverified.set_id(new_id);
    }
}
