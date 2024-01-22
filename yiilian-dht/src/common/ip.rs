use std::convert::TryInto;
use std::net::Ipv4Addr;

#[derive(Clone, Debug)]
/// 群体投票决策权重的IP
struct IPV4Vote {
    ip: Ipv4Addr,
    votes: i32,
}

/// An IPV4Source that takes a certain number of "votes" from other nodes on the network to make its decision.
/// 有群体投票决策权重的 IP 列表, votes.first() 就是投票最多的 IP
#[derive(Clone, Debug)]
pub struct IPV4Consensus {
    min_votes: usize,
    max_votes: usize,
    votes: Vec<IPV4Vote>, // 本机 IP 和 被投票数
}

impl IPV4Consensus {
    pub fn new(min_votes: usize, max_votes: usize) -> IPV4Consensus {
        IPV4Consensus {
            min_votes,
            max_votes,
            votes: Vec::new(),
        }
    }
}

impl IPV4Consensus {
    /// Retrieves the IPv4 address that the source thinks we should have,
    /// or None if it can't make a determination at this time.
    ///
    /// This method will be called periodically by the DHT. Implementations
    /// should return their current best guess for the external (globally routable) IPv4 address
    /// of the DHT.
    /// 
    /// 该方法将被 DHT 定期调用。 返回当前最佳猜测的本机外网（全局可路由）IPv4 地址。
    /// 取出被投票数最多的外网 ipv4 地址，如果获取的投票数没超过阈值，则返回 None
    pub fn get_best_ipv4(&self) -> Option<Ipv4Addr> {
        let first = self.votes.first();
        match first {
            Some(vote_info) => {
                // log::debug!(target: "rustydht_lib::IPV4AddrSource", "Best IPv4 address {:?} has {} votes", vote_info.ip, vote_info.votes);
                if vote_info.votes >= self.min_votes.try_into().unwrap() {
                    Some(vote_info.ip)
                } else {
                    None
                }
            }

            None => None,
        }
    }

    /// Adds a "vote" from another node in the DHT in respose to our queries.
    ///
    /// DHT will call this method when it receive a "hint" from another DHT node
    /// about our external IPv4 address. An IPV4AddrSource implementation can
    /// use these "hints" or "votes", or ignore them.
    ///
    /// # Parameters
    /// * `their_addr` - The IP address of the DHT node that we're learning this information from.
    /// * `proposed_addr` - The external IP address that the other DHT node says we have.
    /// 投票并排序，proposed_addr 是被投票的本机（外网） IP 
    pub fn add_vote(&mut self, _: Ipv4Addr, proposed_addr: Ipv4Addr) {
        let mut do_sort = false;
        for vote in self.votes.iter_mut() {
            if vote.ip == proposed_addr {
                // votes 数取 max_votes（最大投票上限数）和 实际投票数较小的那个值
                vote.votes = std::cmp::min(self.max_votes.try_into().unwrap(), vote.votes + 1);
                do_sort = true;
                break;
            }
        }

        if do_sort {
            // 降序排列
            self.votes.sort_by(|a, b| b.votes.cmp(&a.votes));
        } else {
            self.votes.push(IPV4Vote {
                ip: proposed_addr,
                votes: 1,
            });
        }
    }

    /// This will get called by DHT at some regular interval. Implementations
    /// can use it to allow old information to "decay" over time.
    pub fn decay(&mut self) {
        for vote in self.votes.iter_mut() {
            vote.votes = std::cmp::max(0, vote.votes - 1);
        }

        // Optimize this if we care (hint: we probably don't)
        // 优化，只保留投票数 > 0 的
        self.votes.retain(|a| a.votes > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_src() {
        let mut src = IPV4Consensus::new(2, 4);
        // Nothing yet
        assert_eq!(None, src.get_best_ipv4());

        // One vote, but not enough
        src.add_vote(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(1, 1, 1, 1));
        assert_eq!(None, src.get_best_ipv4());

        // Competing vote, still nothing
        src.add_vote(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(2, 2, 2, 2));
        assert_eq!(None, src.get_best_ipv4());

        // Another vote for the first one. Got something now
        src.add_vote(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(1, 1, 1, 1));
        assert_eq!(Some(Ipv4Addr::new(1, 1, 1, 1)), src.get_best_ipv4());

        // Another vote for the second one. Should still return the first one because in this house our sorts are stable
        src.add_vote(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(2, 2, 2, 2));
        assert_eq!(Some(Ipv4Addr::new(1, 1, 1, 1)), src.get_best_ipv4());

        // Dark horse takes the lead
        src.add_vote(Ipv4Addr::new(0, 0, 0, 0), Ipv4Addr::new(2, 2, 2, 2));
        assert_eq!(Some(Ipv4Addr::new(2, 2, 2, 2)), src.get_best_ipv4());

        // Decay happens
        src.decay();

        // Dark horse still winning
        assert_eq!(Some(Ipv4Addr::new(2, 2, 2, 2)), src.get_best_ipv4());

        // Decay happens again
        src.decay();

        // Nobody wins now
        assert_eq!(None, src.get_best_ipv4());
    }
}
