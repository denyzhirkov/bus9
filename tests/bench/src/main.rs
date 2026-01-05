use clap::{Parser, Subcommand};
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::Duration;

#[derive(Parser)]
#[command(name = "bus9-bench")]
#[command(about = "Load testing tool for Bus9")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Number of concurrent clients
    #[arg(short, long, default_value_t = 50)]
    concurrency: usize,

    /// Total requests to run
    #[arg(short, long, default_value_t = 10000)]
    requests: usize,

    /// Target host
    #[arg(long, default_value = "http://localhost:8080")]
    host: String,
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

    let start = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));

    let stream = stream::iter(0..cli.requests);
    
    stream.for_each_concurrent(cli.concurrency, |_| {
        let client = client.clone();
        let host = cli.host.clone();
        let pb = pb.clone();
        let counter = counter.clone();
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
    let total = counter.load(Ordering::Relaxed);
    let rps = total as f64 / duration.as_secs_f64();

    println!("\n--- Benchmark Results ---");
    println!("Total Requests: {}", total);
    println!("Duration:       {:.2?}", duration);
    println!("Throughput:     {:.2} req/sec", rps);
    println!("Concurrency:    {}", cli.concurrency);
}
