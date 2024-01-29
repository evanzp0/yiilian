use std::{
    collections::HashMap,
    net::SocketAddr,
    num::NonZeroUsize,
    panic::{RefUnwindSafe, UnwindSafe},
    time::Duration,
};

use chrono::{DateTime, Utc};
use lru::LruCache;
use once_cell::sync::OnceCell;
use yiilian_core::{
    common::error::Error, data::{Request, Response}, except_option, except_result, service::{Layer, Service}
};

use crate::common::context::{dht_ctx_routing_tbl, dht_ctx_settings};

pub static mut TRACK_STATE_MAP: OnceCell<HashMap<u16, TrackState>> = OnceCell::new();

#[derive(Clone)]
pub struct FirewallService<S> {
    local_addr: SocketAddr, 
    limit_per_sec: i64,
    inner: S,
}

impl<F> FirewallService<F> {
    pub fn new(inner: F, local_addr: SocketAddr, max_tracks: usize, limit_per_sec: i64) -> Self {
        unsafe {
            TRACK_STATE_MAP.get_or_init(|| {
                HashMap::new()
            });

            let local_port = local_addr.port();
            let map = except_option!(TRACK_STATE_MAP.get_mut(), "Get TRACK_STATE_MAP failed");
            if map.get(&local_port).is_none() {
                let track_state = TrackState::new(max_tracks);
                map.insert(local_port, track_state);
            }
        };

        FirewallService {
            local_addr, 
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
        let ctx_index = self.local_addr.port();
        let is_blocked = except_result!(dht_ctx_routing_tbl(ctx_index).lock(), "Lock routing_table failed")
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

        let local_port = self.local_addr.port();
        let track_state_map = unsafe {
            except_option!(TRACK_STATE_MAP.get_mut(),"Get TRACK_STATE_MAP failed in FirewallService.call()")
        };
        
        if let Some(track_state) = track_state_map.get_mut(&local_port) {
            track_state.add_track_times(req.remote_addr);

            let over_limit = track_state.is_over_limit(req.remote_addr, self.limit_per_sec);
    
            if let Some((is_over_limit, track)) = over_limit {
                log::trace!(
                    target: "yiilian_dht::service::firewall_service",
                    "[{}] address {} request {} times, rps: {}",
                    req.local_addr.port(), req.remote_addr, track.access_times, track.rps()
                );
    
                // 超出防火墙限制，加入黑名单并返回
                if is_over_limit {
                    let block_sec = dht_ctx_settings(ctx_index).firewall_block_duration_sec;
                    except_result!(dht_ctx_routing_tbl(ctx_index).lock(), "Lock context routing_table error")
                        .add_block_list(
                            req.remote_addr,
                            None,
                            Some(Duration::from_secs(block_sec)),
                        );
    
                    let e = Error::new_block(&format!(
                        "address: {:?}, rps: {}",
                        req.remote_addr,
                        track.rps()
                    ));
    
                    log::debug!(
                        target: "yiilian_dht::service::firewall_service", 
                        "[{}] Firewall block address: {}, access {} times, rps: {}", 
                        req.local_addr.port(), req.remote_addr, track.access_times, track.rps());
                    Err(e)?
                }
            }
        }

        self.inner.call(req).await
    }
}

#[derive(Debug)]
pub struct TrackState {
    track_cache: LruCache<SocketAddr, AccessTrack>,
}

impl TrackState {
    fn new(max_tracks: usize) -> TrackState {
        let track_cache = LruCache::new(
            NonZeroUsize::new(max_tracks).expect("Failed to init LruCache for RecvQueryState"),
        );

        TrackState { track_cache }
    }

    /// 增加 addr 对应 track 上的访问次数，如果 track 不存在，则新建一个 track
    fn add_track_times(&mut self, track_addr: SocketAddr) {
        let track_exist = self.get_track(track_addr).is_some();
        if track_exist {
            if let Some(track) = self.get_track_mut(track_addr) {
                track.add_times();
            }
        } else {
            self.inser_track(track_addr);
        }
    }

    fn inser_track(&mut self, track_addr: SocketAddr) {
        let track = AccessTrack::new(track_addr);
        self.track_cache.put(track_addr, track);
    }

    fn get_track_mut(&mut self, track_addr: SocketAddr) -> Option<&mut AccessTrack> {
        let track = self.track_cache.get_mut(&track_addr);

        track
    }

    fn get_track(&mut self, track_addr: SocketAddr) -> Option<&AccessTrack> {
        let track = self.track_cache.get(&track_addr);

        track
    }

    /// 返回 None 意味着对应 address 没有 track 记录
    fn is_over_limit(
        &mut self,
        remote_addr: SocketAddr,
        limit_per_sec: i64,
    ) -> Option<(bool, AccessTrack)> {
        if let Some(track) = self.get_track(remote_addr) {
            if track.access_times <= limit_per_sec {
                Some((false, track.clone()))
            } else if track.rps() > limit_per_sec as f64 {
                Some((true, track.clone()))
            } else {
                Some((false, track.clone()))
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
struct AccessTrack {
    _addr: SocketAddr,
    window_begin_time: DateTime<Utc>,
    update_time: DateTime<Utc>,
    access_times: i64,
}

impl AccessTrack {
    fn new(_addr: SocketAddr) -> Self {
        let now = Utc::now();
        AccessTrack {
            _addr,
            window_begin_time: now,
            update_time: now,
            access_times: 1,
        }
    }

    fn add_times(&mut self) -> &mut Self {
        self.access_times += 1;
        self.update_time = Utc::now();

        self
    }

    fn rps(&self) -> f64 {
        let now = Utc::now();
        let elapsed = (now - self.window_begin_time).num_milliseconds();
        if elapsed > 0 {
            let tps = (self.access_times as f64 / elapsed as f64) * 1000.0;
            println!("{} {}", self.access_times, elapsed);
            tps
        } else {
            0.0
        }
    }
}

pub struct FirewallLayer {
    local_addr: SocketAddr, 
    max_tracks: usize,
    limit_qps: i64,
}

impl FirewallLayer {
    pub fn new(local_addr: SocketAddr, max_tracks: usize, limit_qps: i64) -> Self {
        FirewallLayer {
            local_addr,
            max_tracks,
            limit_qps,
        }
    }
}

impl<F> Layer<F> for FirewallLayer {
    type Service = FirewallService<F>;

    fn layer(&self, inner: F) -> Self::Service {
        FirewallService::new(inner, self.local_addr, self.max_tracks, self.limit_qps)
    }
}
