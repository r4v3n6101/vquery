use std::time::Duration;
use vquery::*;

const ADDR: &'static str = "62.140.250.10:27015";

#[test]
fn a2s_info_old() {
    let query = ValveQuery::<GoldsrcParser>::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    query.set_timeout(Some(Duration::new(10, 0))).unwrap();
    query.connect(ADDR.parse().unwrap()).unwrap();
    println!("{:?}", query.a2s_info_old().unwrap());
}

#[test]
fn a2s_player() {
    let query = ValveQuery::<GoldsrcParser>::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    query.set_timeout(Some(Duration::new(10, 0))).unwrap();
    query.connect(ADDR.parse().unwrap()).unwrap();
    let challenge = query.a2s_player_challenge().unwrap();
    let answer = query.a2s_players(challenge).unwrap();
    println!("{}", challenge);
    println!("{:?}", answer);
}

#[test]
fn a2s_rules() {
    let query = ValveQuery::<GoldsrcParser>::bind("0.0.0.0:0".parse().unwrap()).unwrap();
    query.set_timeout(Some(Duration::new(10, 0))).unwrap();
    query.connect(ADDR.parse().unwrap()).unwrap();
    let challenge = query.a2s_rules_challenge().unwrap();
    let answer = query.a2s_rules(challenge).unwrap();
    println!("{}", challenge);
    println!("{:?}", answer);
}
