use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use std::io::Write;
use std::fs::File;
use std::sync::{mpsc, Arc};
use std::thread;

struct WebsiteStatus {
    url: String,
    action_status: Result<u16, String>,
    response_time: std::time::Duration,
    timestamp: DateTime<Utc>,
}

struct Config {
    file: Option<String>,
    urls: Vec<String>,
    workers: usize,
    timeout_secs: u64,
    retries: usize,
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut args = args.iter().skip(1); 
    let mut file = None;
    let mut urls = Vec::new();
    let mut workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let mut timeout_secs = 5;
    let mut retries = 3;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--file" => {
                file = Some(args.next().ok_or("Missing value after --file")?);
            }
            "--workers" => {
                let n = args.next().ok_or("Missing value after --workers")?;
                workers = n.parse().map_err(|_| "Invalid value for --workers")?;
            }
            "--timeout" => {
                let t = args.next().ok_or("Missing value after --timeout")?;
                timeout_secs = t.parse().map_err(|_| "Invalid value for --timeout")?;
            }
            "--retries" => {
                let r = args.next().ok_or("Missing value after --retries")?;
                retries = r.parse().map_err(|_| "Invalid value for --retries")?;
            }
            _ if arg.starts_with("--") => {
                return Err(format!("Unknown flag: {arg}"));
            }
            _ => {
                urls.push(arg.to_string());
            }
        }
    }

    if file.is_none() && urls.is_empty() {
        return Err("Please specify --file <path> or one or more URLs.".to_string());
    }

    Ok(Config {
        file: file.cloned(),
        urls,
        workers,
        timeout_secs,
        retries,
    })
}

fn read_urls_from_file(path: &str) -> Result<Vec<String>, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect())
}

fn check_website(
    client: &Client,
    url: &str,
    timeout: Duration,
    retries: u32,
) -> WebsiteStatus {
    let start = Instant::now();
    let mut last_err = None;

    for _ in 0..=retries {
        let response = client.get(url).timeout(timeout).send();
        match response {
            Ok(resp) => {
                let duration = start.elapsed();
                return WebsiteStatus {
                    url: url.to_string(),
                    action_status: Ok(resp.status().as_u16()),
                    response_time: duration,
                    timestamp: Utc::now(),
                };
            }
            Err(err) => {
                last_err = Some(err);
            }
        }
    }

    WebsiteStatus {
        url: url.to_string(),
        action_status: Err(format!(
            "Error: {}", 
            last_err.map_or_else(|| "unknown error".to_string(), |e| e.to_string()))),
        response_time: start.elapsed(),
        timestamp: Utc::now(),
    }
}





fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config = parse_args(&args).expect("Invalid arguments");

    let mut urls = config.urls.clone();
    if let Some(file_path) = &config.file {
        let mut file_urls = read_urls_from_file(file_path).expect("Failed to read URLs from file");
        urls.append(&mut file_urls);
    }

    let client = Arc::new(Client::new());
    let (tx, rx) = mpsc::channel();
    let urls = Arc::new(urls); // Share URLs with threads

    // Distribute the work
    for i in 0..config.workers {
        let tx = tx.clone();
        let client = Arc::clone(&client);
        let urls = Arc::clone(&urls);
        let timeout = Duration::from_secs(config.timeout_secs);
        let retries = config.retries as u32;

        thread::spawn(move || {
            for j in (i..urls.len()).step_by(config.workers) {
                let url = &urls[j];
                let status = check_website(&client, url, timeout, retries);
                tx.send(status).expect("Failed to send result");
            }
        });
    }

    drop(tx); // Close the sending end so the receiver can finish

    // Collect results
    let mut statuses = Vec::new();
    for received in rx {
        statuses.push(received);
    }

    let status_strings: Vec<String> = statuses
        .iter()
        .map(|status| {
            format!(
                "{{\"url\": \"{}\", \"status\": \"{}\", \"response_time\": \"{}\", \"timestamp\": \"{}\"}}",
                status.url,
                status.action_status.as_ref().map_or("unknown error".to_string(), |s| s.to_string()),
                status.response_time.as_secs(),
                status.timestamp.to_rfc3339()
            )
        })
        .collect();

    let final_output = format!("[{}]", status_strings.join(",\n"));

    let mut file = File::create("status.json").expect("Unable to create file");
    file.write_all(final_output.as_bytes()).expect("Unable to write data");

    println!("Output written to status_output.txt");
}





#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::blocking::Client;
    use std::time::Duration;

    #[test]
    fn test_parse_args_valid() {
        let args = vec![
            "program_name".to_string(),
            "--workers".to_string(),
            "4".to_string(),
            "--timeout".to_string(),
            "10".to_string(),
            "--retries".to_string(),
            "3".to_string(),
            "https://example.com".to_string(),
            assert_eq!(config.timeout_secs, 10);
            assert_eq!(config.retries, 3); 
        ];

        let result = parse_args(&args);
        assert!(result.is_ok());
        let config = result.unwrap();

        assert_eq!(config.workers, 4);
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.retries, 3);
        assert_eq!(config.urls.len(), 1);
        assert_eq!(config.urls[0], "https://example.com");
    }

    #[test]
    fn test_parse_args_missing_url() {
        let args = vec![
            "program_name".to_string(),
            "--workers".to_string(),
            "4".to_string(),
        ];

        let result = parse_args(&args);

        assert!(result.is_err());
    }

    #[test]
    fn test_check_website_success() {
        let client = Client::new();
        let url = "https://example.com";
        let status = check_website(&client, url, Duration::from_secs(5), 3);
        assert!(status.action_status.is_ok());
        assert_eq!(status.url, url.to_string());
    }
}