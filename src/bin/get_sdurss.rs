use rss::Channel;

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    let positions = sdu_openrss::get_open_positions().await.unwrap();
    let channel = Channel {
        title: "Open Positions @ SDU".to_owned(),
        link: sdu_openrss::OPEN_POSITIONS.to_owned(),
        description: "Read and parsed".to_owned(),
        language: Some("en".to_owned()),
        generator: Some("sdu_openrss".to_owned()),
        items: positions.into_iter().map(|p| (&p).into()).collect(),
        last_build_date: Some(chrono::Utc::now().to_rfc2822()),
        webmaster: Some("markus.horst.becker+sdu@gmail.com (Markus Horst Becker)".to_string()),
        ..Channel::default()
    };
    channel.pretty_write_to(std::io::stdout(), b' ', 4)?;
    Ok(())
}
