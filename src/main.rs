use std::{collections::HashMap, process::Stdio, time::Duration};
use serde::Serialize;

use fantoccini::{Client, ClientBuilder, Locator};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type NumberMap = HashMap<String, HashMap<String, Player>>;

#[derive(Debug, Serialize, Clone)]
struct Player {
    hof: bool,
    teams: Vec<Team>,
}

#[derive(Debug, Serialize, Clone)]
struct Team {
    name: String,
    seasons: Vec<u16>,
}

#[tokio::main]
async fn main() -> Result {
    let _cmd = tokio::process::Command::new("geckodriver")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let c = ClientBuilder::rustls()
        .capabilities(serde_json::from_str(r#"{"moz:firefoxOptions": {"args": ["--headless"]}}"#)?)
        .connect("http://localhost:4444")
        .await?;
    let mut all = NumberMap::new();
    let number_results = run_for(&c, "00").await.unwrap();
    std::fs::write("./data/00.json", serde_json::to_string_pretty(&number_results)?)?;
    all.insert("00".to_string(), number_results);
    for i in 0..100 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let number = i.to_string();
        let number_results = run_for(&c, &number).await?;
        std::fs::write(format!("./data/{number}.json"), serde_json::to_string_pretty(&number_results)?)?;
        all.insert(number, number_results);
    }
    let out = serde_json::to_string_pretty(&all)?;
    std::fs::write("all.json", &out)?;
    Ok(())
}

async fn run_for(c: &Client, number: &str) -> Result<HashMap<String, Player>> {
    eprintln!("running for {number}");
    c.goto(&format!(
        "https://www.basketball-reference.com/friv/numbers.cgi?number={number}"
    ))
    .await?;
    let mut ret: HashMap<String, Player> = HashMap::new();
    let eles = c.find_all(Locator::Css("tr[data-row]")).await?;
    for ele in eles {
        let lefts = ele.find_all(Locator::Css(".left")).await?;
        let mut left = lefts[0].text().await?;
        let hof = left.trim().ends_with('*');
        left = left.trim().trim_end_matches('*').to_string();
        let right = lefts[1].text().await?;
        let mut player = count_seasons(&right);
        player.hof = hof;
        ret.entry(left).and_modify(|v| {
            v.teams.extend(player.teams.clone());
        }).or_insert(player);
    }
    Ok(ret)
}

fn count_seasons(right: &str) -> Player {
    let teams = right.trim().lines().filter_map(|l| {
        let start_seasons = l.find('(').or_else(|| {
            eprintln!("bad season line in {right:?}");
            None
        })?;
        let name = l.get(..start_seasons)?.trim().to_string();
        let seasons = l.get(start_seasons..)?.trim().trim_start_matches('(').trim_end_matches(')').split(',').filter_map(|n| {
            let t = n.trim();
            if t.starts_with(&['0', '1', '2']) {
                return format!("20{t}").parse().inspect_err(|e| eprintln!("Error parsing year {n:?}(20{t}): {e}")).ok()
            }
            format!("19{t}").parse().inspect_err(|e| eprintln!("Error parsing year {n:?}(19{t}): {e}")).ok()
        }).collect();
        Some(Team {
            name,
            seasons,
        })
    }).collect();
    Player {
        hof: false,
        teams,
    }
}
