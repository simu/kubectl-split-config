use anyhow::anyhow;
use saphyr::{Hash, Yaml};

pub fn read_list<'k>(kubeconfig: &'k Hash, key: &str) -> anyhow::Result<&'k Vec<Yaml>> {
    kubeconfig
        .get(&Yaml::String(key.to_owned()))
        .ok_or(anyhow!("key '{key}' missing?"))?
        .as_vec()
        .ok_or(anyhow!("{key} not a list?"))
}

pub fn read_string<'h>(obj: &'h Hash, key: &str) -> anyhow::Result<&'h str> {
    obj.get(&Yaml::String(key.to_owned()))
        .ok_or(anyhow!("key '{key}' missing"))?
        .as_str()
        .ok_or(anyhow!("{key} not a string?"))
}

pub fn find_entry<'l>(list: &'l Vec<Yaml>, name: &str) -> anyhow::Result<Option<&'l Yaml>> {
    for e in list {
        let n = e
            .as_hash()
            .ok_or(anyhow!("list entry not a hash"))?
            .get(&Yaml::String("name".to_owned()));
        if n == Some(&Yaml::String(name.to_owned())) {
            return Ok(Some(e));
        }
    }
    Ok(None)
}
