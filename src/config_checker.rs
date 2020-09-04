use {
    anyhow::bail,
    clap::YamlLoader,
    lazy_static::lazy_static,
    std::collections::{BTreeMap, HashMap},
    yaml_rust::Yaml,
};

const REQUIRED_KEYS: [&str; 1] = ["name"];

lazy_static! {
    static ref BANNED_KEYS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("help", "about");
        m
    };
}

pub fn required(yaml_config: &BTreeMap<Yaml, Yaml>) -> anyhow::Result<()> {
    // We must check these ourselves because if you don't specify a name in the YAML config, clap
    // screws up, probably this is a clap bug.
    for key in &REQUIRED_KEYS {
        if !yaml_config.contains_key(&YamlLoader::load_from_str(key).unwrap()[0]) {
            bail!("YAML config must contain an entry named '{}'", key);
        }
    }
    Ok(())
}

pub fn banned(yaml_config: &BTreeMap<Yaml, Yaml>) -> anyhow::Result<()> {
    // If these are present in the YAML config, clap screws up, probably this is a clap bug.
    for (bannedk, suggestion) in &*BANNED_KEYS {
        if yaml_config.contains_key(&YamlLoader::load_from_str(bannedk).unwrap()[0]) {
            bail!(
                "YAML config can't contain an entry named '{}', try '{}'",
                bannedk,
                suggestion
            );
        }
    }
    Ok(())
}
