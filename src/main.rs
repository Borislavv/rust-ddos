mod model;
mod service;

use tokio::signal;
use tokio::sync::mpsc;
use service::ddos_service;
use model::ddos;
use std::{sync::Arc, sync::atomic::AtomicI32};
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use chrono::{Utc, Duration};
use clap::Parser;
use tokio::sync::broadcast;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // number of workers
    #[arg(short, long, default_value_t = 10)]
    workers: i32,
    // duration of ddos
    #[arg(short, long, default_value_t = 5)]
    duration: i64,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let workers = Arc::new(AtomicI32::new(args.workers));
    let duration = args.duration;

    let num_workers = Arc::clone(&workers);
    let (shutdown_tx, shutdown_rx) = broadcast::channel(num_workers.load(Ordering::Relaxed) as usize);
    let mut shutdown_rx2 = shutdown_tx.subscribe();

    let (timeout_tx, mut timeout_rx) = mpsc::channel::<bool>(1);

    let timeout_handler = tokio::spawn(async move {
        let will = Utc::now() + Duration::minutes(duration);

        loop {
            if Utc::now() > will {
                match timeout_tx.send(true).await {
                    Ok(()) => { println!("Timeout exceeded, closing...") },
                    Err(err) => { println!("Timeout exceeded, but error occurred while sending data to chan: {:?}", err.to_string()); },
                }
                return;
            } else if shutdown_rx2.try_recv().unwrap_or(false) {
                println!("timeout: thread closed by ctrl+c");
                return;
            }
        }
    });

    let num_workers = Arc::clone(&workers);

    let ddos_handler = tokio::spawn(
        async move {
            let ddos = ddos_service::DDOSService::new(
                ddos::DDOS::new(
                    "https://seo-swoole-async-pagedata.lux.kube.xbet.lan/api/v1/pagedata?group_id=62
                    &ref_id=8&url=http://melbet.com/en/footboll/1&geo=by&language=en&project[id]=62
                    &domain=melbet.com&stream=live&section=sport&sport[id]=6&timezone=4".to_string(),
                    num_workers.load(std::sync::atomic::Ordering::Relaxed)
                )
            );
        
            ddos.run(Arc::new(Mutex::new(shutdown_rx))).await;
        }
    );

    let num_workers = Arc::clone(&workers);

    let shutdown_handler = tokio::spawn(
        async move {
            tokio::select! {
                _ = timeout_rx.recv() => {
                    println!("timeout exceeded, closing...");
                    for w in 0..num_workers.load(Ordering::SeqCst) {
                        shutdown_tx.send(true).expect(&format!("signal sent to shutdown_tx for worker #{}", w));
                    }
                },
                _ = signal::ctrl_c() => {
                    println!("received CTRL+C (interrupt signal)");
                    for w in 0..num_workers.load(Ordering::SeqCst) {
                        shutdown_tx.send(true).expect(&format!("signal sent to shutdown_tx for worker #{}", w));
                    }
                },
            }
        }
    );

    tokio::try_join!(timeout_handler).expect("timeout handler is joined to the main thread");
    tokio::try_join!(shutdown_handler).expect("shutdown handler is joined to the main thread");
    tokio::try_join!(ddos_handler).expect("ddos handler is joined to the main thread");
}