

use std::env;

use tracing::{event, span, Level};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    let env_path = env::current_dir().and_then(|a| {
        Ok(a.join("yiilian-web/examples/.tracing_it.env"))
    }).unwrap();
    
    dotenv::from_path(env_path.as_path()).unwrap();

    let log_path = env::current_dir().and_then(|a| {
        Ok(a.join("yiilian-web/examples/log"))
    }).unwrap();

    let file_appender = tracing_appender::rolling::daily(log_path, "tracing_it.log");
    // let log_file = std::fs::File::create("log.log").unwrap();
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .with(EnvFilter::from_env("APP_LOG"))
        .init();

    // tracing_subscriber::fmt()
    //     .with_max_level(Level::DEBUG)
    //     .init();

    let span = span!(Level::TRACE, "my_span");
    let _enter = span.enter();
    event!(Level::TRACE, "this is info log");

    println!("{}", my_mod::my_meth("zp"));
    
}

mod my_mod {
    use tracing::{debug, info, trace};

     // 函数名为嵌套 span
    #[tracing::instrument(level = "DEBUG")]
    pub(crate) fn my_meth(name: &str) -> String {

        let mut out = "hello ".to_owned();
        out += name;

        info!(out);
        let pos = Position { x: 3.234, y: -1.223 };

        debug!(?pos.x, ?pos.y);
        trace!(target: "app_events", position = ?pos, "New position");
        trace!(name: "completed", position = ?pos);
        out
    }

    #[derive(Debug)]
    struct Position {
        x: f64,
        y: f64,
    }
}

/*
2024-04-11T08:52:06.179039Z TRACE my_span: tracing_it: this is info log
2024-04-11T08:52:06.179096Z  INFO my_span:my_meth{name="zp"}: tracing_it::my_mod: out="hello zp"
2024-04-11T08:52:06.179123Z  INFO my_span:my_meth{name="zp"}: tracing_it::my_mod: pos.x=3.234 pos.y=-1.223
2024-04-11T08:52:06.179140Z  INFO my_span:my_meth{name="zp"}: app_events: New position position=Position { x: 3.234, y: -1.223 }
2024-04-11T08:52:06.179155Z  INFO my_span:my_meth{name="zp"}: tracing_it::my_mod: position=Position { x: 3.234, y: -1.223 }
*/


// use tracing::{debug, info, span, Level};
// // use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
// fn main() {
//     // tracing_subscriber::registry().with(fmt::layer()).init();
//     tracing_subscriber::fmt()
//         .with_max_level(Level::DEBUG)  // 低于配置的 level 的 span 会显示（但不影响 event 的显示），低于 level 的 整个 event 不显示
//         .init();

//     let scope = span!(Level::TRACE, "foo");
//     let _enter = scope.enter();
//     info!("Hello in foo scope");
//     debug!("before entering bar scope"); 
//     {
//         let scope = span!(Level::INFO, "bar", ans = 42);
//         let _enter = scope.enter();
//         debug!("enter bar scope");
//         info!("In bar scope");
//         debug!("end bar scope");
//     }
//     debug!("end bar scope");
// }