use anyhow::anyhow;
use saphyr::{Mapping, Yaml};

pub fn read_list<'k>(kubeconfig: &'k Mapping, key: &'k str) -> anyhow::Result<&'k Vec<Yaml<'k>>> {
    kubeconfig
        .get(&Yaml::value_from_str(key))
        .ok_or(anyhow!("key '{key}' missing?"))?
        .as_vec()
        .ok_or(anyhow!("{key} not a list?"))
}

pub fn read_string<'h>(obj: &'h Mapping, key: &'h str) -> anyhow::Result<&'h str> {
    obj.get(&Yaml::value_from_str(key))
        .ok_or(anyhow!("key '{key}' missing"))?
        .as_str()
        .ok_or(anyhow!("{key} not a string?"))
}

pub fn find_entry<'l>(list: &'l Vec<Yaml<'l>>, name: &str) -> anyhow::Result<Option<&'l Yaml<'l>>> {
    for e in list {
        let n = e
            .as_mapping()
            .ok_or(anyhow!("list entry not a hash"))?
            .get(&Yaml::value_from_str("name"));
        if n == Some(&Yaml::value_from_str(name)) {
            return Ok(Some(e));
        }
    }
    Ok(None)
}
