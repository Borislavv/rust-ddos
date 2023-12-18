mod model;
mod service;

use tokio::signal;
use tokio::sync::mpsc;
use service::ddos_service;
use model::ddos;
use std::{time, sync::Arc, sync::Mutex};
use chrono::{Utc, Duration};

#[tokio::main]
async fn main() {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<bool>(1);
    let (timeout_tx, mut timeout_rx) = mpsc::channel::<bool>(1);

    tokio::spawn(async move {
        let will = Utc::now() + Duration::minutes(5);

        loop {
            if Utc::now() > will {
                timeout_tx.send(true).await.unwrap();
            }
            tokio::time::sleep(time::Duration::from_secs(1)).await;
        }
    });

    let ddos_task = tokio::spawn(
        async move {
            let ddos = ddos_service::DDOSService::new(
                ddos::DDOS::new(
                    "https://seo-php-swoole.lux.kube.xbet.lan/api/v1/pagedata?group_id=285&
                    url=php-swoole-test-domain.com/fr&geo=by&language=fr&project[id]=285&domain=
                    php-swoole-test-domain.com&timezone=3&stream=live&section=sport&sport[id]=1".to_string(), 
                    100
                )
            );
        
            ddos.run(Arc::new(Mutex::new(shutdown_rx))).await;
        }
    );

    let shutdown = tokio::spawn(
        async move {
            tokio::select! {
                _ = timeout_rx.recv() => {
                    println!("timeout exceeded");
                    for _ in 0..=100 {
                        shutdown_tx.send(true).await.unwrap();
                    }
                },
                _ = signal::ctrl_c() => {
                    println!("received CTRL+C (interrupt signal)");
                    for _ in 0..=100 {
                        shutdown_tx.send(true).await.unwrap();
                    }
                },
                _ = ddos_task => {}
            }
        }
    );

    tokio::try_join!(shutdown).unwrap();
}