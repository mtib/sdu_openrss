#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let positions = sdu_openrss::get_open_positions().await?;
    for p in positions.into_iter() {
        println!("{:?} {}: {} {}", p.deadline, p.campus, p.title, p.faculty);
    }
    Ok(())
}
