use std::collections::HashMap;

use anyhow::anyhow;
use saphyr::{Mapping, Yaml, YamlEmitter};

use crate::yaml::*;

/// Check if file is a Kubeconfig file by looking at `kind` and `apiVersion`
pub fn is_kubeconfig(data: &Yaml) -> bool {
    if let Some(kubeconfig) = data.as_mapping() {
        if let Some(apiversion) = kubeconfig.get(&Yaml::value_from_str("apiVersion")) {
            if let Some(kind) = kubeconfig.get(&Yaml::value_from_str("kind")) {
                if apiversion.as_str() == Some("v1") && kind.as_str() == Some("Config") {
                    return true;
                }
            }
        }
    }
    false
}

fn read_context<'c>(ctx: &'c Yaml<'c>) -> anyhow::Result<(&'c Yaml<'c>, &'c Mapping<'c>)> {
    let ctxdata = ctx.as_mapping().ok_or(anyhow!("Context not a hash"))?;
    let ctxname = ctxdata
        .get(&Yaml::value_from_str("name"))
        .ok_or(anyhow!("context has no name?"))?;
    let context = ctxdata
        .get(&Yaml::value_from_str("context"))
        .ok_or(anyhow!("Context has no field 'context'"))?
        .as_mapping()
        .ok_or(anyhow!("Field 'context' not a hash"))?;
    Ok((ctxname, context))
}

pub struct Kubeconfig<'c> {
    config: Mapping<'c>,
}

impl<'c> Kubeconfig<'c> {
    fn new() -> Self {
        let mut k = Self {
            config: Mapping::new(),
        };
        k.config.insert(
            Yaml::value_from_str("apiVersion"),
            Yaml::value_from_str("v1"),
        );
        k.config
            .insert(Yaml::value_from_str("kind"), Yaml::value_from_str("Config"));
        k.config
            .insert(Yaml::value_from_str("clusters"), Yaml::Sequence(vec![]));
        k.config
            .insert(Yaml::value_from_str("contexts"), Yaml::Sequence(vec![]));
        k.config
            .insert(Yaml::value_from_str("users"), Yaml::Sequence(vec![]));
        k
    }

    fn add_context(
        &mut self,
        ctxname: &'c Yaml,
        ctx: &'c Yaml,
        cluster: &'c Yaml,
        user: &'c Yaml,
        current: bool,
    ) -> anyhow::Result<()> {
        let contexts = self.config[&Yaml::value_from_str("contexts")]
            .as_sequence_mut()
            .ok_or(anyhow!("contexts not vec?"))?;
        if contexts.contains(ctx) {
            return Err(anyhow!("Kubeconfig already contains context {ctx:?}"));
        }
        contexts.push(ctx.clone());
        let clusters = self.config[&Yaml::value_from_str("clusters")]
            .as_sequence_mut()
            .ok_or(anyhow!("clusters not vec?"))?;
        if !clusters.contains(cluster) {
            clusters.push(cluster.clone());
        }
        let users = self.config[&Yaml::value_from_str("users")]
            .as_sequence_mut()
            .ok_or(anyhow!("users not vec?"))?;
        if !users.contains(user) {
            users.push(user.clone());
        }

        if current {
            self.config
                .insert(Yaml::value_from_str("current-context"), ctxname.clone());
        }

        Ok(())
    }

    pub fn write(&self, emitter: &mut YamlEmitter) -> anyhow::Result<()> {
        emitter
            .dump(&Yaml::Mapping(self.config.clone()))
            .map_err(|e| anyhow!("{e}"))
    }
}

pub fn split_into_contexts<'c>(
    kubeconfig: &'c Yaml<'c>,
    output_file_pattern: &str,
    skip_string: &Option<String>,
) -> anyhow::Result<HashMap<String, Kubeconfig<'c>>> {
    let mut res = HashMap::new();

    let data = kubeconfig
        .as_mapping()
        .ok_or(anyhow!("Not a kubeconfig?"))?;
    let contexts = read_list(data, "contexts")?;
    let clusters = read_list(data, "clusters")?;
    let users = read_list(data, "users")?;

    for ctx in contexts {
        let (ctxname, ctxdata) = read_context(ctx)?;
        let ctxname_str = ctxname
            .as_str()
            .ok_or(anyhow!("context name not a string"))?;
        if skip_string
            .as_ref()
            .is_some_and(|p| ctxname_str.contains(p))
        {
            println!(
                "Skipping context {ctxname_str} which matches {}",
                skip_string.as_ref().unwrap_or(&"".to_owned())
            );
            continue;
        }
        let cluster_name = read_string(ctxdata, "cluster")?;
        let cluster = find_entry(clusters, cluster_name)?.ok_or(anyhow!(
            "Cluster '{cluster_name}' not present in kubeconfig"
        ))?;
        let user_name = read_string(ctxdata, "user")?;
        let user = find_entry(users, user_name)?
            .ok_or(anyhow!("User '{user_name}' not present in kubeconfig"))?;
        let mut kubeconfig = Kubeconfig::new();
        kubeconfig.add_context(ctxname, ctx, cluster, user, true)?;
        let default_ns = Yaml::scalar_from_string("default".into());
        let namespace = ctxdata
            .get(&Yaml::value_from_str("namespace"))
            .unwrap_or(&default_ns)
            .as_str()
            .ok_or(anyhow!("namespace not string"))?;
        let fname = output_file_pattern.replace("CLUSTER", &cluster_name.replace('/', "_"));
        let fname = fname.replace("NAMESPACE", namespace);
        let fname = fname.replace("USER", &user_name.replace('/', "_"));
        let prev = res.insert(fname, kubeconfig);
        if prev.is_some() {
            return Err(anyhow!("Provided output file name pattern is not unique"));
        }
    }

    Ok(res)
}
