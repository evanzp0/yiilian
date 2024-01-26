use std::{
    collections::HashMap,
    net::SocketAddr,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::time::sleep;
use yiilian_core::{
    common::{
        error::Error,
        shutdown::{spawn_with_shutdown, ShutdownReceiver},
        util::hash_it,
    },
    data::{Request, Response},
    service::{Layer, Service},
};

use crate::common::context::Context;

pub struct FirewallService<S> {
    ctx: Arc<Context>,
    rqs_state: Arc<RwLock<RecvQueryState>>,
    limit_per_sec: i64,
    inner: S,
}

impl<F> FirewallService<F> {
    pub fn new(
        inner: F,
        ctx: Arc<Context>,
        window_size_sec: i64,
        limit_per_sec: i64,
        shutdown: ShutdownReceiver,
    ) -> Self {
        let rqs_state = RecvQueryState::new(window_size_sec, shutdown);

        FirewallService {
            ctx,
            rqs_state,
            limit_per_sec,
            inner,
        }
    }
}

impl<S, B1, B2> Service<Request<B1>> for FirewallService<S>
where
    S: Service<Request<B1>, Response = Response<B2>, Error = Error> + Send + Sync + RefUnwindSafe,
    B1: Send + UnwindSafe,
{
    type Response = S::Response;
    type Error = S::Error;

    async fn call(&self, req: Request<B1>) -> Result<Self::Response, Self::Error> {
        let is_blocked = self
            .ctx
            .routing_table()
            .lock()
            .expect("lock routing_table failed")
            .is_blocked(&req.remote_addr);
        if is_blocked {
            log::debug!(
                target: "yiilian_dht::service::firewall_service", 
                "Address is blocked: [{}] {:?}", 
                req.local_addr.port(), req.remote_addr
            );

            let e = Error::new_block(&format!("Address is blocked: {:?}", req.remote_addr));
            Err(e)?
        }

        let track_id = self
            .rqs_state
            .write()
            .unwrap()
            .add_track_times(&req.remote_addr);

        let over_limit = self
            .rqs_state
            .write()
            .unwrap()
            .is_over_limit(track_id, self.limit_per_sec);
        if let Some((is_over_limit, track)) = over_limit {
            log::trace!(
                target: "yiilian_dht::service::firewall_service",
                "[{}] address {} request {} times, rps: {}",
                req.local_addr.port(), req.remote_addr, track.access_times, track.rps(self.limit_per_sec)
            );

            // 超出防火墙限制，加入黑名单并返回
            if is_over_limit {
                let block_sec = self.ctx.settings().firewall_block_duration_sec;
                self.ctx.routing_table().lock().unwrap().add_block_list(
                    req.remote_addr,
                    None,
                    Some(Duration::from_secs(block_sec)),
                    self.ctx.clone(),
                );

                let e = Error::new_block(&format!(
                    "address: {:?}, rps: {}",
                    req.remote_addr,
                    track.rps(self.limit_per_sec)
                ));

                log::debug!(target: "yiilian_dht::service::firewall_service", "[{}] Firewall block address: {}, access {} times, rps: {}", 
                    req.local_addr.port(), req.remote_addr, track.access_times, track.rps(self.limit_per_sec));
                Err(e)?
            }
        }

        self.inner.call(req).await
    }
}

#[derive(Debug)]
struct RecvQueryState {
    window_size_sec: i64,
    track_map: HashMap<u64, AccessTrack>,
}

impl RecvQueryState {
    fn new(window_size_sec: i64, shutdown: ShutdownReceiver) -> Arc<RwLock<RecvQueryState>> {
        let track_map = HashMap::new();

        let rs = RecvQueryState {
            window_size_sec,
            track_map,
        };
        let rs = Arc::new(RwLock::new(rs));
        let rs1 = rs.clone();

        spawn_with_shutdown(
            shutdown,
            async move {
                loop {
                    rs1.write().unwrap().prune();
                    sleep(Duration::from_secs(20)).await;
                }
            },
            "firewall service prune loop",
            None,
        );

        rs
    }

    /// 增加 addr 对应 track 上的访问次数，如果 track 不存在，则新建一个 track
    fn add_track_times(&mut self, addr: &SocketAddr) -> u64 {
        let track_id = hash_it(addr);
        let track_exist = self.get_track(track_id).is_some();

        if track_exist {
            self.get_track_mut(hash_it(addr))
                .expect("add_track_times should got the track")
                .add_times()
                .id()
        } else {
            self.inser_track(addr).id()
        }
    }

    fn inser_track(&mut self, addr: &SocketAddr) -> &mut AccessTrack {
        let track = AccessTrack::new(addr);
        let track_id = track.id();
        self.track_map.insert(track_id, track);

        self.track_map
            .get_mut(&track_id)
            .expect("inser_track should return track")
    }

    fn get_track_mut(&mut self, track_id: u64) -> Option<&mut AccessTrack> {
        let track = self.track_map.get_mut(&track_id);

        track
    }

    fn get_track(&self, track_id: u64) -> Option<&AccessTrack> {
        let track = self.track_map.get(&track_id);

        track
    }

    /// 返回 None 意味着对应 address 没有 track 记录
    fn is_over_limit(&self, track_id: u64, limit_per_sec: i64) -> Option<(bool, AccessTrack)> {
        if let Some(track) = self.get_track(track_id) {
            if track.access_times <= 10 {
                Some((false, track.clone()))
            } else if track.rps(limit_per_sec) > limit_per_sec {
                Some((true, track.clone()))
            } else {
                Some((false, track.clone()))
            }
        } else {
            None
        }
    }

    fn prune(&mut self) {
        let mut rst: Vec<u64> = vec![];
        let now = Utc::now();
        for (key, val) in &mut self.track_map {
            if (now - val.update_time).num_seconds() > self.window_size_sec {
                rst.push(*key);
            } else {
                val.reset();
            }
        }

        rst.iter().for_each(|key| {
            self.track_map.remove(key);
        });
    }
}

#[derive(Debug, Clone)]
struct AccessTrack {
    id: u64,
    window_begin_time: DateTime<Utc>,
    update_time: DateTime<Utc>,
    access_times: i64,
}

impl AccessTrack {
    fn new(addr: &SocketAddr) -> Self {
        let now = Utc::now();
        let id = hash_it(addr);
        AccessTrack {
            id,
            window_begin_time: now,
            update_time: now,
            access_times: 1,
        }
    }

    fn id(&self) -> u64 {
        self.id
    }

    fn add_times(&mut self) -> &mut Self {
        self.access_times += 1;
        self.update_time = Utc::now();

        self
    }

    fn reset(&mut self) -> &mut Self {
        self.window_begin_time = self.update_time;
        self.access_times = 1;

        self
    }

    fn rps(&self, limit_per_sec: i64) -> i64 {
        if self.access_times <= limit_per_sec {
            return self.access_times;
        }

        let now = Utc::now();
        let elapsed = (now - self.window_begin_time).num_milliseconds();
        if elapsed > 0 {
            let tps = ((self.access_times as f64 / elapsed as f64) * 1000.0) as i64;

            tps
        } else {
            i64::MAX
        }
    }
}

pub struct FirewallLayer {
    ctx: Arc<Context>,
    window_size_sec: i64,
    limit_per_sec: i64,
    shutdown: ShutdownReceiver,
}

impl FirewallLayer {
    pub fn new(
        ctx: Arc<Context>,
        window_size_sec: i64,
        limit_per_sec: i64,
        shutdown: ShutdownReceiver,
    ) -> Self {
        FirewallLayer {
            ctx,
            window_size_sec,
            limit_per_sec,
            shutdown: shutdown,
        }
    }
}

impl<F> Layer<F> for FirewallLayer {
    type Service = FirewallService<F>;

    fn layer(&self, inner: F) -> Self::Service {
        FirewallService::new(
            inner,
            self.ctx.clone(),
            self.window_size_sec,
            self.limit_per_sec,
            self.shutdown.clone(),
        )
    }
}
