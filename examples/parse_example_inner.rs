#[tokio::main]
async fn main() {
    let openings = sdu_openrss::parse_dom(
        &String::from_utf8(std::fs::read("examples/example_inner.html").unwrap()).unwrap(),
    )
    .unwrap();
    println!("{:?}", openings);
}
