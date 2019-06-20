use tagtime::scheduler::State;

fn main() {
    let s = State::from_millis(1533812000000);
    dbg!(s
        .take(5)
        .map(|x| x.timestamp_millis() / 100)
        .collect::<Vec<_>>());
}
