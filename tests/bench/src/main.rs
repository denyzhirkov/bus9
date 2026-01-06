use clap::{Parser, Subcommand};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[derive(Parser)]
#[command(name = "bus9-bench")]
#[command(about = "Load testing tool for Bus9")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Number of concurrent publishers
    #[arg(short, long, default_value_t = 50)]
    concurrency: usize,

    /// Total requests to publish
    #[arg(short, long, default_value_t = 10000)]
    requests: usize,

    /// Target host
    #[arg(long, default_value = "http://localhost:8080")]
    host: String,

    /// Number of concurrent subscribers (Pub/Sub)
    #[arg(long, default_value_t = 0)]
    subscribers: usize,

    /// Number of concurrent consumers (Queue)
    #[arg(long, default_value_t = 0)]
    consumers: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Benchmark Pub/Sub publishing
    Pub {
        #[arg(long, default_value = "bench-topic")]
        topic: String,
    },
    /// Benchmark Queue pushing
    Queue {
        #[arg(long, default_value = "bench-queue")]
        name: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let client = Client::new();

    let pb = ProgressBar::new(cli.requests as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
        .unwrap()
        .progress_chars("#>-"));

    let received_count = Arc::new(AtomicUsize::new(0));
    let consumed_count = Arc::new(AtomicUsize::new(0));
    let mut bg_tasks = Vec::new();

    // Start Subscribers (Pub/Sub)
    if cli.subscribers > 0 {
        if let Commands::Pub { topic } = &cli.command {
            println!("Starting {} subscribers on topic '{}'...", cli.subscribers, topic);
            let ws_base = cli.host.replace("http", "ws");
            let url = format!("{}/api/sub?topic={}", ws_base, topic);
            
            for _ in 0..cli.subscribers {
                let url = url.clone();
                let counter = received_count.clone();
                let handle = tokio::spawn(async move {
                    if let Ok((mut ws_stream, _)) = connect_async(&url).await {
                        while let Some(msg) = ws_stream.next().await {
                            if let Ok(Message::Text(_)) = msg {
                                counter.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                });
                bg_tasks.push(handle);
            }
            // Give subscribers time to connect
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    // Start Consumers (Queue)
    if cli.consumers > 0 {
        if let Commands::Queue { name } = &cli.command {
            println!("Starting {} consumers on queue '{}'...", cli.consumers, name);
            let target_url = format!("{}/api/queue/{}", cli.host, name);
            
            for _ in 0..cli.consumers {
                let client = client.clone();
                let url = target_url.clone();
                let counter = consumed_count.clone();
                let handle = tokio::spawn(async move {
                    loop {
                        if let Ok(res) = client.get(&url).send().await {
                            if res.status().is_success() && res.status() != reqwest::StatusCode::NO_CONTENT {
                                counter.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        // Yield to avoid starving stats/publisher
                        tokio::task::yield_now().await;
                    }
                });
                bg_tasks.push(handle);
            }
        }
    }

    // Run Publisher Benchmark
    let start = Instant::now();
    let sent_counter = Arc::new(AtomicUsize::new(0));

    let stream = stream::iter(0..cli.requests);
    
    stream.for_each_concurrent(cli.concurrency, |_| {
        let client = client.clone();
        let host = cli.host.clone();
        let pb = pb.clone();
        let counter = sent_counter.clone();
        let cmd = match &cli.command {
            Commands::Pub { topic } => ("pub", topic.clone()),
            Commands::Queue { name } => ("queue", name.clone()),
        };

        async move {
            let url = match cmd.0 {
                "pub" => format!("{}/api/pub?topic={}", host, cmd.1),
                "queue" => format!("{}/api/queue/{}", host, cmd.1),
                _ => unreachable!(),
            };

            let res = client.post(&url)
                .body("benchmark payload 1234567890")
                .send()
                .await;

            if let Ok(resp) = res {
                if resp.status().is_success() {
                    counter.fetch_add(1, Ordering::Relaxed);
                }
            }
            pb.inc(1);
        }
    }).await;

    pb.finish_with_message("Done");
    let duration = start.elapsed();
    let total_sent = sent_counter.load(Ordering::Relaxed);
    let rps = total_sent as f64 / duration.as_secs_f64();

    // Wait a bit for subscribers to drain
    if cli.subscribers > 0 || cli.consumers > 0 {
        println!("Waiting for clients to finish receiving...");
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    println!("\n--- Benchmark Results ---");
    println!("Type:           {:?}", match cli.command { Commands::Pub{..} => "Pub/Sub", Commands::Queue{..} => "Queue" });
    println!("Duration:       {:.2?}", duration);
    println!("Sent:           {} (Throughput: {:.2} req/sec)", total_sent, rps);
    
    if cli.subscribers > 0 {
        let received = received_count.load(Ordering::Relaxed);
        println!("Received (WS):  {}", received);
    }
    if cli.consumers > 0 {
        let consumed = consumed_count.load(Ordering::Relaxed);
        println!("Consumed (GET): {}", consumed);
    }
}
