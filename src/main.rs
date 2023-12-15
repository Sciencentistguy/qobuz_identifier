use clap::Parser;
use once_cell::sync::Lazy;
use reqwest::{Client, Method};
use serde::Deserialize;
use serde_json::Value;

use color_eyre::eyre::Result;
use url::Url;

static ARGS: Lazy<Args> = Lazy::new(Args::parse);

const QOBUZ_APP_ID: &str = "712109809";

#[derive(clap::Parser)]
struct Args {
    // The url (or Qobuz ID) to identify
    url: String,
}

// "https://www.qobuz.com/api.json/0.2/album/get?album_id=e488xvkt1tw9a&offset=0&limit=20"

async fn api_req(client: &Client, id: &str) -> Result<Value> {
    let request = client
        .request(Method::GET, "https://www.qobuz.com/api.json/0.2/album/get")
        .query(&[("album_id", id), ("offset", "0"), ("limit", "20")])
        .header("x-app-id", QOBUZ_APP_ID)
        .build()?;
    let response = client.execute(request).await?;
    let json = response.json().await?;
    Ok(json)
}

struct MbSearchResult {
    id: String,
    name: String,
    ac: String,
}

async fn mb_search(client: &Client, upc: &str) -> Result<Vec<String>> {
    let req = client
        .request(Method::GET, "https://musicbrainz.org/ws/2/release")
        .query(&[("query", format!("barcode:{upc}"))])
        .header("User-Agent", "Mozilla/5.0 (example@example.org)")
        .header("accept", "application/json")
        .build()?;
    let response = client.execute(req).await?;
    let json: Value = response.json().await?;
    Ok(json
        .get("releases")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result.get("id").unwrap().as_str().unwrap().to_owned())
        .collect())
}

/// a UPC is 12 characters. Often, Qobuz will produce barcodes that:
///  - miss the last digit - the "check digit"
///  - start with an extra "00"
fn validate_upc(mut upc: &str) -> bool {
    if !upc.is_ascii() {
        return false;
    }

    if upc.starts_with("00") {
        upc = &upc[2..];
    }

    let mut chars: Vec<_> = upc.chars().collect();

    let expected_check = if upc.len() == 12 {
        // has check digit
        let Some(check) = chars.pop().unwrap().to_digit(10) else {
            return false;
        };
        Some(check)
    } else if upc.len() == 11 {
        // missing check digit
        None
    } else {
        return false;
    };

    let mut total = 0;

    for (i, char) in chars
        .iter()
        .rev()
        .enumerate()
        .map(|(i, char)| (i + 1, char))
    {
        let Some(digit) = char.to_digit(10) else {return false};
        // if i is even, 3, else 1.
        let x = if (i) & 1 == 0 { 3 } else { 1 };
        total += digit * x;
    }

    let actual_check = 10 - (total % 10) % 10;

    if let Some(expected) = expected_check {
        expected == actual_check
    } else {
        true
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let url = Url::parse(&ARGS.url)?;
    let id = url
        .path_segments()
        .expect("url should have path segments")
        .last()
        .unwrap();

    let client = Client::new();
    let response = api_req(&client, id).await?;

    let error = matches!(response.get("status").and_then(|x| x.as_str()), Some("error"));
    if error {
        eprintln!("Error response: {response}");
        return Ok(());
    }
    let upc = response.get("upc").unwrap();
    let upc = upc.as_str().unwrap();

    let results = mb_search(&client, upc).await?;

    println!("Detected barcode '{upc}'.");
    if !results.is_empty() {
        println!("Matched {} MBIDs", results.len());
        for id in results {
            println!("https://beta.musicbrainz.org/release/{id}");
        }
    }

    Ok(())
}
