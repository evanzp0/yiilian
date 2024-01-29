use std::{
    net::SocketAddr,
    num::NonZeroUsize,
    panic::{RefUnwindSafe, UnwindSafe},
    sync::{Arc, RwLock},
    time::Duration,
};

use chrono::{DateTime, Utc};
use lru::LruCache;
use crate::{
    common::{error::Error, shutdown::ShutdownReceiver},
    data::{Request, Response},
    except_result,
    net::block_list::BlockList,
    service::{Layer, Service},
};

pub const BLOCK_SEC: u64 = 60 * 60 * 8;

#[derive(Clone)]
pub struct FirewallService<S> {
    track_state: Arc<RwLock<TrackState>>,
    block_list: Arc<RwLock<BlockList>>,
    limit_per_sec: i64,
    inner: S,
}

unsafe impl<F> Send for FirewallService<F> {}
unsafe impl<F> Sync for FirewallService<F> {}
impl<F> RefUnwindSafe for FirewallService<F> {}

impl<F> FirewallService<F> {
    pub fn new(
        inner: F,
        max_tracks: usize,
        limit_per_sec: i64,
        block_list_max_size: Option<i32>,
        shutdown_rx: ShutdownReceiver,
    ) -> Self {
        let track_state = Arc::new(RwLock::new(TrackState::new(max_tracks)));
        let block_list = Arc::new(RwLock::new(BlockList::new(block_list_max_size.unwrap_or(65535), None, shutdown_rx)));

        except_result!(block_list.read(), "block_list.read() error").prune_loop();

        FirewallService {
            track_state,
            block_list,
            limit_per_sec,
            inner,
        }
    }

    /// 判断 addr 是否在黑名单中
    pub fn is_blocked(&self, addr: &SocketAddr) -> bool {
        except_result!(self.block_list.read(), "block_list.read() error").contains(addr.ip(), addr.port())
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
        let is_blocked = self.is_blocked(&req.remote_addr);
        let local_port = req.local_addr.port();

        if is_blocked {
            log::debug!(
                target: "yiilian_core::service::firewall_service",
                "Address is blocked: [{}] {:?}",
                local_port, req.remote_addr
            );

            let e = Error::new_block(&format!("Address is blocked: {:?}", req.remote_addr));
            Err(e)?
        }

        // if let Some(track_state) = track_state_map.get_mut(&local_port) {
        except_result!(self.track_state.write(), "track_state.write() error")
            .add_track_times(req.remote_addr);
        self.track_state.write().expect("").add_track_times(req.remote_addr);

        // let over_limit = except_result!(self.track_state.read(), "track_state.write() error")
        //     .is_over_limit(req.remote_addr, self.limit_per_sec);
        let over_limit = self.track_state.write().expect("").is_over_limit(req.remote_addr, self.limit_per_sec);

        if let Some((is_over_limit, track)) = over_limit {
            log::trace!(
                target: "yiilian_dht::service::firewall_service",
                "[{}] address {} request {} times, rps: {}",
                req.local_addr.port(), req.remote_addr, track.access_times, track.rps()
            );

            // 超出防火墙限制，加入黑名单并返回
            if is_over_limit {
                except_result!(self.block_list.write(), "block_list.write() error").insert(
                    req.remote_addr.ip(),
                    req.remote_addr.port() as i32,
                    Some(Duration::from_secs(BLOCK_SEC)),
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
        // }

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
    max_tracks: usize,
    limit_per_sec: i64,
    block_list_max_size: Option<i32>,
    shutdown_rx: ShutdownReceiver,
}

impl FirewallLayer {
    pub fn new(
        max_tracks: usize,
        limit_per_sec: i64,
        block_list_max_size: Option<i32>,
        shutdown_rx: ShutdownReceiver,
    ) -> Self {
        FirewallLayer {
            max_tracks,
            limit_per_sec,
            block_list_max_size,
            shutdown_rx,
        }
    }
}

impl<F> Layer<F> for FirewallLayer {
    type Service = FirewallService<F>;

    fn layer(&self, inner: F) -> Self::Service {
        FirewallService::new(
            inner,
            self.max_tracks,
            self.limit_per_sec,
            self.block_list_max_size,
            self.shutdown_rx.clone(),
        )
    }
}
