use reqwest::Client;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Testing Polymarket API...");
    
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    
    let url = "https://gamma-api.polymarket.com/markets?limit=1&closed=false";
    println!("Fetching: {}", url);
    
    let response = client.get(url).send().await?;
    println!("Status: {}", response.status());
    
    let text = response.text().await?;
    println!("Response length: {} bytes", text.len());
    println!("First 200 chars: {}", &text[..200.min(text.len())]);
    
    Ok(())
}
