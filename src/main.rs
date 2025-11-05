use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;
use reqwest::header::{HeaderMap, HeaderValue, HeaderName};
use std::str::FromStr;
use colored::*;
use regex::Regex;
use std::env;
use dotenvy;
use once_cell::sync::Lazy;
use futures::stream::{FuturesUnordered, StreamExt};

static ENV_VAR_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{\{(\w+)\}\}").unwrap());

fn substitute_env_vars(text: &str) -> String {
    ENV_VAR_REGEX.replace_all(text, |caps: &regex::Captures| {
        let var_name = &caps[1];
        env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
    }).to_string()
}

#[derive(Parser, Debug)]
#[command(version, about = "JSON-based HTTP Request CLI")]
struct Args {
    #[arg(short, long)]
    file: String,
    
    #[arg(short, long, default_value = "30")]
    timeout: u64,
    
    #[arg(short, long, value_parser = ["pretty", "json"])]
    output: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct RequestSpec {
    name: Option<String>,
    url: String,
    method: String,
    headers: Option<std::collections::HashMap<String, String>>,
    params: Option<std::collections::HashMap<String, String>>,
    body: Option<serde_json::Value>,
    form: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Serialize, Clone)]
struct RequestResult {
    name: String,
    url: String,
    method: String,
    status_code: Option<u16>,
    status_text: Option<String>,
    success: bool,
    response_time_ms: f64,
    response_body: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestSummary {
    total: usize,
    success: usize,
    failed: usize,
    success_rate: f64,
    results: Vec<RequestResult>,
}

async fn process_request(client: reqwest::Client, req: RequestSpec, timeout: u64) -> RequestResult {
    let request_name = req.name.as_deref().unwrap_or("Unnamed").to_string();

    let mut builder = match req.method.to_uppercase().as_str() {
        "GET" => client.get(&req.url),
        "POST" => client.post(&req.url),
        "PUT" => client.put(&req.url),
        "DELETE" => client.delete(&req.url),
        "PATCH" => client.patch(&req.url),
        _ => {
            return RequestResult {
                name: request_name,
                url: req.url.clone(),
                method: req.method.clone(),
                status_code: None,
                status_text: None,
                success: false,
                response_time_ms: 0.0,
                response_body: None,
                error: Some(format!("Unsupported method: {}", req.method)),
            };
        }
    };

    if let Some(headers) = &req.headers {
        let mut header_map = HeaderMap::new();
        for (k, v) in headers {
            if let (Ok(name), Ok(value)) = (HeaderName::from_str(k), HeaderValue::from_str(v)) {
                header_map.insert(name, value);
            }
        }
        builder = builder.headers(header_map);
    }

    if let Some(params) = &req.params {
        builder = builder.query(params);
    }

    if let Some(body) = &req.body {
        if req.form.is_some() {
            return RequestResult {
                name: request_name,
                url: req.url.clone(),
                method: req.method.clone(),
                status_code: None,
                status_text: None,
                success: false,
                response_time_ms: 0.0,
                response_body: None,
                error: Some("Cannot use 'body' and 'form' fields simultaneously.".to_string()),
            };
        }
        builder = builder.json(body);
    } else if let Some(form) = &req.form {
        builder = builder.form(form);
    }

    let start = std::time::Instant::now();
    let response = builder.send().await;
    let elapsed = start.elapsed();
    let response_time_ms = elapsed.as_secs_f64() * 1000.0;

    match response {
        Ok(resp) => {
            let status = resp.status();
            let status_code = status.as_u16();
            let status_text = status.canonical_reason().unwrap_or("").to_string();
            let is_success = status.is_success();
            
            let text = resp.text().await.unwrap_or_default();
            let response_body = serde_json::from_str::<serde_json::Value>(&text).ok();
            
            RequestResult {
                name: request_name,
                url: req.url.clone(),
                method: req.method.clone(),
                status_code: Some(status_code),
                status_text: Some(status_text),
                success: is_success,
                response_time_ms,
                response_body,
                error: None,
            }
        }
        Err(err) => {
            let error_msg = if err.is_timeout() {
                format!("Request timeout ({}s)", timeout)
            } else if err.is_connect() {
                "Unable to connect to server".to_string()
            } else if err.is_request() {
                "Invalid request".to_string()
            } else if err.is_body() {
                "Body processing failed".to_string()
            } else if err.is_decode() {
                "Response decoding failed".to_string()
            } else {
                "Unknown error".to_string()
            };
            
            RequestResult {
                name: request_name,
                url: req.url.clone(),
                method: req.method.clone(),
                status_code: None,
                status_text: None,
                success: false,
                response_time_ms,
                response_body: None,
                error: Some(format!("{}: {}", error_msg, err)),
            }
        }
    }
}

fn print_result(result: &RequestResult, total_requests: usize, request_index: usize) {
    println!("\n{} {}", 
        format!("[{}/{}]", request_index, total_requests).bright_cyan(),
        result.name.bright_white().bold()
    );
    println!("{} {} {}", 
        "Method:".bright_black(),
        result.method.to_uppercase().bright_yellow(),
        result.url.bright_black()
    );

    if let Some(status_code) = result.status_code {
        let status_text = result.status_text.as_deref().unwrap_or("");
        let status_display = if result.success {
            format!("✅ Status: {} {}", status_code, status_text).green()
        } else if status_code >= 400 && status_code < 500 {
            format!("⚠️  Status: {} {}", status_code, status_text).yellow()
        } else if status_code >= 500 {
            format!("❌ Status: {} {}", status_code, status_text).red()
        } else {
            format!("ℹ️  Status: {} {}", status_code, status_text).blue()
        };
        println!("{}", status_display);
    }

    println!("{} {:.2}s", "Response time:".bright_black(), result.response_time_ms / 1000.0);

    if let Some(error) = &result.error {
         println!("{} {}", "❌ Error:".red().bold(), error.bright_black());
    }

    println!("\n{}", "Response body:".bright_white().bold());
    if let Some(json) = &result.response_body {
        let pretty = serde_json::to_string_pretty(json).unwrap_or_default();
        if pretty.len() > 500 {
            println!("{}", &pretty[..500].bright_black());
            println!("{}", format!("... ({} bytes truncated)", pretty.len() - 500).bright_black().italic());
        } else {
            println!("{}", pretty.bright_black());
        }
    } else {
        println!("{}", "(empty)".bright_black());
    }
    println!("{}", "-".repeat(60).bright_black());
}

fn print_summary_box(total: usize, success: usize, failed: usize, success_rate: f64, failed_requests: Vec<String>) {
    let mut lines = vec![
        format!("Total: {}", total),
        format!("Success: {}", success),
        format!("Failed: {}", failed),
        format!("Success rate: {:.1}%", success_rate),
    ];

    if !failed_requests.is_empty() {
        lines.push("".to_string());
        lines.push("Failed Requests:".to_string());
        for name in failed_requests {
            lines.push(format!("  - {}", name));
        }
    }

    let max_line_width = lines.iter().map(|s| unicode_width::UnicodeWidthStr::width(s.as_str())).max().unwrap_or(0);
    let title = "Test Summary";
    let title_width = unicode_width::UnicodeWidthStr::width(title);
    let box_width = std::cmp::max(max_line_width, title_width) + 4;

    println!("\n┌{}┐", "─".repeat(box_width));
    let padding_total = box_width - title_width;
    let padding_left = padding_total / 2;
    let padding_right = padding_total - padding_left;
    println!("│{}{}{}│", " ".repeat(padding_left), title, " ".repeat(padding_right));
    println!("├{}┤", "─".repeat(box_width));
    
    for line in lines {
        let content = format!("  {}", line);
        let line_width = unicode_width::UnicodeWidthStr::width(content.as_str());
        let padding = " ".repeat(box_width - line_width);
        println!("│{}{}│", content, padding);
    }

    println!("└{}┘", "─".repeat(box_width));
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args = Args::parse();
    let data = fs::read_to_string(&args.file)?;
    let substituted_data = substitute_env_vars(&data);
    let requests: Vec<RequestSpec> = serde_json::from_str(&substituted_data)?;

    let output_json = args.output.as_deref() == Some("json");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(args.timeout))
        .build()?;

    if !output_json {
        println!("{}", "=".repeat(60).bright_blue());
        println!("{}", format!("HTTP Request Test Started (Timeout: {}s)", args.timeout).bright_blue().bold());
        println!("{}", "=".repeat(60).bright_blue());
    }

    let mut futures = FuturesUnordered::new();
    for req in requests.clone() {
        let client = client.clone();
        futures.push(tokio::spawn(process_request(client, req, args.timeout)));
    }

    let mut results = Vec::new();
    let total_requests = requests.len();
    let mut request_index = 0;
    while let Some(result) = futures.next().await {
        let result = result.unwrap();
        request_index += 1;
        if !output_json {
            print_result(&result, total_requests, request_index);
        }
        results.push(result);
    }

    let success_count = results.iter().filter(|r| r.success).count();
    let fail_count = results.len() - success_count;
    let success_rate = if !requests.is_empty() {
        (success_count as f64 / requests.len() as f64) * 100.0
    } else {
        0.0
    };

    let failed_requests: Vec<String> = results
        .iter()
        .filter(|r| !r.success)
        .map(|r| r.name.clone())
        .collect();

    if output_json {
        let summary = TestSummary {
            total: requests.len(),
            success: success_count,
            failed: fail_count,
            success_rate,
            results,
        };
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_summary_box(requests.len(), success_count, fail_count, success_rate, failed_requests);
    }

    Ok(())
}