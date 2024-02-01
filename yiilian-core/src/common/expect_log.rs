
pub trait ExpectLog<T> {
    fn expect_error(self, msg: &str) -> T;
    fn expect_warn(self, msg: &str) -> T;
    fn expect_info(self, msg: &str) -> T;
    fn expect_debug(self, msg: &str) -> T;
    fn expect_trace(self, msg: &str) -> T;
}

impl<T> ExpectLog<T> for Option<T> {
    fn expect_error(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => {
                log::error!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_warn(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => {
                log::warn!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_info(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => {
                log::info!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_debug(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => {
                log::debug!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_trace(self, msg: &str) -> T {
        match self {
            Some(val) => val,
            None => {
                log::trace!("{msg}");
                panic!("{msg}")
            },
        }
    }
}

impl<T, E> ExpectLog<T> for Result<T, E> {
    fn expect_error(self, msg: &str) -> T {
        match self {
            Ok(val) => val,
            Err(_) => {
                log::error!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_warn(self, msg: &str) -> T {
        match self {
            Ok(val) => val,
            Err(_) => {
                log::warn!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_info(self, msg: &str) -> T {
        match self {
            Ok(val) => val,
            Err(_) => {
                log::info!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_debug(self, msg: &str) -> T {
        match self {
            Ok(val) => val,
            Err(_) => {
                log::debug!("{msg}");
                panic!("{msg}")
            },
        }
    }

    fn expect_trace(self, msg: &str) -> T {
        match self {
            Ok(val) => val,
            Err(_) => {
                log::trace!("{msg}");
                panic!("{msg}")
            },
        }
    }
}