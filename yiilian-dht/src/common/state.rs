
use super::{id::Id, ip::IPV4Consensus};

#[derive(Debug)]
/// 存放当前 DHT 的各项状态参数
pub struct State {
    /// local node id
    local_id: Id,

    /// 有群体投票决策权重的 IP 列表
    pub ip4_source: IPV4Consensus,

    /// 当前生成 token 的密钥
    pub token_secret: Vec<u8>,

    /// 上次生成 token 的密钥
    pub old_token_secret: Vec<u8>,

    /// 是否已加入 kad
    pub is_join_kad: bool,
}

impl State {
    pub fn new(
        local_id: Id,
        ip4_source: IPV4Consensus,
        token_secret: Vec<u8>,
    ) -> Self {
        State {
            local_id,
            ip4_source,
            old_token_secret: token_secret.clone(),
            token_secret,
            is_join_kad: false,
        }
    }

    pub fn get_local_id(&self) -> Id {
        self.local_id
    }

    pub fn set_local_id(&mut self, local_id: Id) {
        self.local_id = local_id;
    }
}
