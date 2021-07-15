use tagview::settings::{RunConfig};

fn main() {
    let x = "name = \"test_settings_serde\"
    time_limit = 3600";

    let de: RunConfig = toml::de::from_str(x).unwrap();


    println!("{:?}", de);
}