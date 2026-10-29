//! API binary entrypoint.

use std::backtrace::Backtrace;
use std::panic::PanicHookInfo;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn main() {
    // Init tracing.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();

    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info: &PanicHookInfo<'_>| {
        eprintln!("=== API PANIC START ===");
        eprintln!("{info}");
        eprintln!("backtrace: {}", Backtrace::force_capture());
        eprintln!("=== API PANIC END ===");
        original_hook(info);
    }));

    let result = std::panic::catch_unwind(|| {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async_main())
    });

    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("API error: {e:?}");
            std::process::exit(1);
        }
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };
            eprintln!("Uncaught panic: {msg}");
            std::process::exit(101);
        }
    }
}

async fn async_main() -> anyhow::Result<()> {
    let config = api::Config::load()?;
    api::run(config).await
}
