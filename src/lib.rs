#[allow(unused_imports)]
use std::hash::Hasher;
#[allow(unused_imports)]
use std::io::Write;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    convert::TryFrom,
    fmt::Display,
    fs::File,
    hash::Hash,
    time::Duration,
};

use chrono::{DateTime, FixedOffset, Local, NaiveDate};
use failure::bail;
use fantoccini::{ClientBuilder, Locator};
use html_parser::{Dom, Element, Node};
use rss::{Guid, Item, Source};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use tokio::time::sleep;
use webdriver::capabilities::Capabilities;

const DOMAIN_ROOT: &str = "https://www.sdu.dk";
pub const OPEN_POSITIONS: &str = "https://www.sdu.dk/en/service/ledige_stillinger";
const MEMORY_FILE_LOCATION: &str = ".memory";

pub async fn get_html() -> Result<String, failure::Error> {
    let mut cap: Capabilities = Map::new();
    let mut chrome_options = Map::new();

    chrome_options.insert("args".to_string(), json!(["--headless"]));

    if let Ok(bin) = std::env::var("CHROME_BINARY") {
        chrome_options.insert("binary".to_string(), json!(bin));
    }

    cap.insert(
        "goog:chromeOptions".to_string(),
        Value::Object(chrome_options),
    );

    let mut cl = ClientBuilder::native()
        .capabilities(cap)
        .connect("http://localhost:9515")
        .await
        .map_err(|_| failure::format_err!("Connection to Chromedriver on Port 9515 failed"))?;

    cl.goto(OPEN_POSITIONS).await?;

    sleep(Duration::from_secs(5)).await;

    let mut table = cl.find(Locator::Css("tbody.list")).await?;

    let inner_html = table.html(false).await?;

    cl.close().await?;
    Ok(inner_html)
}

#[derive(Debug, Hash)]
pub enum Campus {
    Copenhagen,
    Esbjerg,
    Kolding,
    Odense,
    Slagelse,
    Soenderborg,
    Several,
}

impl From<&Campus> for String {
    fn from(c: &Campus) -> Self {
        match c {
            Campus::Soenderborg => "Sønderborg".to_owned(),
            _ => format!("{:?}", c),
        }
    }
}

impl Display for Campus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl TryFrom<&str> for Campus {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Copenhagen" => Ok(Campus::Copenhagen),
            "Esbjerg" => Ok(Campus::Esbjerg),
            "Kolding" => Ok(Campus::Kolding),
            "Odense" => Ok(Campus::Odense),
            "Slagelse" => Ok(Campus::Slagelse),
            "Sønderborg" => Ok(Campus::Soenderborg),
            "Flere tjenestesteder" => Ok(Campus::Several),
            _ => bail!("Campus not known: {}", value),
        }
    }
}

#[derive(Debug, Hash)]
pub enum Faculty {
    Engineering,
    HealthSciences,
    BusinessAndSocialSciences,
    Science,
    Humanities,
    CentralAdministration,
    Library,
}

impl TryFrom<&str> for Faculty {
    type Error = failure::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Faculty of Engineering" => Ok(Faculty::Engineering),
            "Faculty of Health Sciences" => Ok(Faculty::HealthSciences),
            "Faculty of Business and Social Sciences" => Ok(Faculty::BusinessAndSocialSciences),
            "Faculty of Science" => Ok(Faculty::Science),
            "Faculty of Humanities" => Ok(Faculty::Humanities),
            "Central Administration" => Ok(Faculty::CentralAdministration),
            "SDU Library" => Ok(Faculty::Library),
            _ => bail!("Faculty not known: {}", value),
        }
    }
}

impl From<&Faculty> for String {
    fn from(c: &Faculty) -> Self {
        match c {
            Faculty::Engineering => "Faculty of Engineering",
            Faculty::HealthSciences => "Faculty of Health Sciences",
            Faculty::BusinessAndSocialSciences => "Faculty of Business and Social Sciences",
            Faculty::Science => "Faculty of Science",
            Faculty::Humanities => "Faculty of Humanities",
            Faculty::CentralAdministration => "Central Administration",
            Faculty::Library => "SDU Library",
        }
        .to_owned()
    }
}

impl Display for Faculty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl Faculty {
    pub fn to_short_name(&self) -> &str {
        match self {
            Faculty::Engineering => "Engineering",
            Faculty::HealthSciences => "Health",
            Faculty::BusinessAndSocialSciences => "Business",
            Faculty::Science => "Science",
            Faculty::Humanities => "Humanities",
            Faculty::CentralAdministration => "Administration",
            Faculty::Library => "Library",
        }
    }
}

#[derive(Debug)]
pub struct Position {
    pub link: String,
    pub title: String,
    pub campus: Campus,
    pub deadline: NaiveDate,
    pub faculty: Faculty,
    pub first_seen: Option<DateTime<FixedOffset>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Memory {
    first_seen_map: HashMap<u64, String>,
}

impl Default for Memory {
    fn default() -> Self {
        Memory {
            first_seen_map: HashMap::new(),
        }
    }
}

const DEADLINE_FORMAT: &str = "%Y-%B-%d";

impl Hash for Position {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.link.hash(state);
        self.title.hash(state);
        self.campus.hash(state);
        self.faculty.hash(state);
    }
}

impl TryFrom<&Element> for Position {
    type Error = failure::Error;

    fn try_from(element: &Element) -> Result<Self, failure::Error> {
        let (link, title) = if let Node::Element(td1) = &element.children[0] {
            if let Some(Node::Element(a)) = &td1.children.get(0) {
                let s = a.attributes.get("href").cloned().unwrap().unwrap();
                let t = if let Some(Node::Text(t)) = &a.children.get(0) {
                    t.split_whitespace().collect::<Vec<&str>>().join(" ")
                } else {
                    bail!("No title in anchor: {:?}", a);
                };
                (s, t)
            } else {
                bail!("Unexpected inner structure: {:?}", td1);
            }
        } else {
            bail!("Unexpected outer structure: {:?}", element);
        };

        let faculty = if let Node::Element(td2) = &element.children[1] {
            if let Some(Node::Text(t)) = td2.children.get(0) {
                t.clone().replace("&amp;", "").replace("&nbsp;", " ")
            } else {
                bail!("Unexpected inner structure: {:?}", td2);
            }
        } else {
            bail!("Unexpected outer structure: {:?}", element);
        };

        let campus = if let Node::Element(td3) = &element.children[2] {
            if let Some(Node::Text(t)) = td3.children.get(0) {
                Campus::try_from(t.as_str()).unwrap()
            } else {
                bail!("Unexpected inner structure: {:?}", td3);
            }
        } else {
            bail!("Unexpected outer structure: {:?}", element);
        };

        let deadline = if let Node::Element(td4) = &element.children[3] {
            if let Some(Node::Text(t)) = td4.children.get(0) {
                NaiveDate::parse_from_str(t.as_str(), DEADLINE_FORMAT).unwrap()
            } else {
                bail!("Unexpected inner structure: {:?}", td4);
            }
        } else {
            bail!("Unexpected outer structure: {:?}", element);
        };
        Ok(Position {
            link: {
                if link.starts_with('/') {
                    format!("{}{}", DOMAIN_ROOT, link)
                } else {
                    link
                }
            },
            title,
            faculty: Faculty::try_from(faculty.as_str())?,
            campus,
            deadline,
            first_seen: None,
        })
    }
}

impl From<&Position> for Item {
    fn from(p: &Position) -> Self {
        //let dt = chrono::Utc.from_utc_datetime(&p.deadline.and_hms(0, 0, 0));
        use ellipse::Ellipse;
        Item {
            title: Some(format!(
                "{}/{} {}: {}",
                p.faculty.to_short_name(),
                String::from(&p.campus),
                p.deadline,
                p.title.as_str().truncate_ellipse(80)
            )),
            link: Some(p.link.clone()),
            description: Some(p.title.clone()),
            pub_date: p.first_seen.map(|t| t.to_rfc2822()),
            source: Some(Source {
                url: OPEN_POSITIONS.to_owned(),
                title: None,
            }),
            guid: Some(Guid {
                value: p.link.clone(),
                permalink: true,
            }),
            ..Item::default()
        }
    }
}

fn recurse_search(element: &Element, positions: &mut Vec<Position>) {
    element
        .children
        .iter()
        .filter_map(|n| match n {
            html_parser::Node::Element(e) => Some(e),
            _ => None,
        })
        .for_each(|e| recurse_search(e, positions));
    if element.name == "tr" {
        if let Ok(p) = Position::try_from(element) {
            positions.push(p)
        }
    }
}

pub fn parse_dom(html: &str) -> Result<Vec<Position>, failure::Error> {
    let dom = Dom::parse(html)?;

    let mut positions = Vec::new();

    dom.children
        .into_iter()
        .filter_map(|n| match n {
            html_parser::Node::Element(e) => Some(e),
            _ => None,
        })
        .for_each(|e| recurse_search(&e, &mut positions));

    let mut memory = {
        if let Ok(f) = File::open(MEMORY_FILE_LOCATION) {
            serde_json::from_reader(f).unwrap_or_else(|_| Memory::default())
        } else {
            Memory::default()
        }
    };

    let positions = positions
        .into_iter()
        .map(|mut p| {
            let mut h = DefaultHasher::new();
            p.hash(&mut h);
            let v = h.finish();
            if let Some(t) = memory.first_seen_map.get(&v) {
                p.first_seen = DateTime::parse_from_rfc2822(t.as_str()).ok();
            } else {
                let now = Local::now();
                memory.first_seen_map.insert(v, now.to_rfc2822());
                p.first_seen = Some(now.into());
            }
            p
        })
        .collect();

    if let Ok(f) = File::create(MEMORY_FILE_LOCATION) {
        serde_json::to_writer(f, &memory).unwrap();
    }

    Ok(positions)
}

pub async fn get_open_positions() -> Result<Vec<Position>, failure::Error> {
    let inner_html = get_html().await?;
    parse_dom(&inner_html)
}
