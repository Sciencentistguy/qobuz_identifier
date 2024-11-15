use std::sync::Arc;

use clap::Parser;
use itertools::Itertools as _;
use once_cell::sync::Lazy;
use reqwest::{Client, Method};
use serde::Deserialize;
use serde_json::Value;

use color_eyre::eyre::Result;
use tokio::task::JoinSet;
use url::Url;

static ARGS: Lazy<Args> = Lazy::new(Args::parse);

#[derive(Clone, Debug, Deserialize)]
struct LoginDetails {
    appid: String,
    username: String,
    password: String,
}

/// A small command-line tool that takes a qobuz ID and matches it to MusicBrainz releases by
/// barcode.
#[derive(clap::Parser)]
struct Args {
    /// The url to identify
    url: Url,

    /// The file to read the qobuz app id from, or the appid itself.
    #[clap(
        long = "qobuz-app-id-file",
        default_value = "/secrets/qobuz_identifier_app_id",
        env = "QBID_DETAILS",
        value_parser = read_qobuz_app_id,
    )]
    login_details: LoginDetails,
}

fn read_qobuz_app_id(path: &str) -> Result<LoginDetails> {
    let str = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&str)?)
}

async fn get_qobuz_album(client: &Client, id: &str, user_auth_token: &str) -> Result<Value> {
    let request = client
        .request(Method::GET, "https://www.qobuz.com/api.json/0.2/album/get")
        .query(&[("album_id", id), ("offset", "0"), ("limit", "20")])
        .header("x-app-id", ARGS.login_details.appid.as_str())
        .header("x-user-auth-token", user_auth_token)
        .build()?;
    let response = client.execute(request).await?;
    let json = response.json().await?;
    Ok(json)
}

async fn mb_search(client: Arc<Client>, barcode: String) -> Result<Vec<String>> {
    let req = client
        .request(Method::GET, "https://musicbrainz.org/ws/2/release")
        .query(&[("query", format!("barcode:{barcode}"))])
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

fn expand_barcode(barcode: &str) -> Vec<String> {
    let mut barcodes = vec![barcode.to_owned()];
    if let Some(barcode) = barcode.strip_prefix("00") {
        barcodes.push(barcode.to_owned()); // strip leading 00, as this can be extraneous
    }
    if let Some(barcode) = barcodes.iter().find(|bc| bc.len() == 13) {
        if let Some(barcode) = barcode.strip_prefix('0') {
            barcodes.push(barcode.to_owned()); // a 13 char barcode starting with a 0 can sometimes be
                                               // equivalent to a 12 char barcode with the 0 removed
        }
    }

    if let Some(barcode) = barcodes.iter().find(|bc| bc.len() == 11) {
        // calculate check digit, append it, and add to search criteria
        let check: u32 = barcode
            .chars()
            .map(|x| x.to_digit(10).unwrap())
            .zip(std::iter::repeat([3, 1]).flatten())
            .map(|(a, b)| a * b)
            .sum();
        let nearest_10_above = ((check + 9) / 10) * 10;
        let check = nearest_10_above - check;
        barcodes.push(format!("{barcode}{check}"));
    }

    barcodes
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install().unwrap();

    let id = ARGS
        .url
        .path_segments()
        .expect("url should have path segments")
        .last()
        .unwrap();

    let client = Arc::new(Client::new());

    let auth_token = {
        let req = client
            .request(Method::GET, "https://www.qobuz.com/api.json/0.2/user/login")
            .header("X-app-id", ARGS.login_details.appid.as_str())
            .header("X-username", ARGS.login_details.username.as_str())
            .header("X-password", ARGS.login_details.password.as_str())
            .build()?;

        let resp: serde_json::Value = client.execute(req).await?.json().await?;

        let serde_json::Value::String(tok) = resp
            .get("user_auth_token")
            .expect("login should return auth token")
        else {
            panic!("user-auth-token should be a string")
        };
        tok.clone()
    };

    let response = get_qobuz_album(&client, id, &auth_token).await?;

    if matches!(
        response.get("status").and_then(|x| x.as_str()),
        Some("error")
    ) {
        eprintln!("Error response: {response}");
        return Ok(());
    }
    let barcode = response.get("upc").unwrap();
    let barcode = barcode.as_str().unwrap();
    let barcodes = expand_barcode(barcode);

    let mut set = JoinSet::new();
    for bc in barcodes {
        set.spawn(mb_search(Arc::clone(&client), bc));
    }

    let mut mbids = Vec::new();
    while let Some(res) = set.join_next().await {
        mbids.extend(res??);
    }

    mbids = mbids.into_iter().unique().collect();

    println!("Detected barcode '{barcode}'.");
    if !mbids.is_empty() {
        println!("Matched {} MBIDs", mbids.len());
        for id in mbids {
            println!("https://beta.musicbrainz.org/release/{id}");
        }
    } else {
        println!("No matching MBIDs were found. Either the correct release does not exist, or the barcode is missing from it.");
    }

    Ok(())
}
