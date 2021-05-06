#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    println!("{}", sdu_openrss::get_html().await?);
    Ok(())
}
