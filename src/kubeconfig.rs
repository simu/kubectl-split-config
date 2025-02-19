use std::collections::HashMap;

use anyhow::anyhow;
use saphyr::{Hash, Yaml, YamlEmitter};

use crate::yaml::*;

/// Check if file is a Kubeconfig file by looking at `kind` and `apiVersion`
pub fn is_kubeconfig(data: &Yaml) -> bool {
    if let Some(kubeconfig) = data.as_hash() {
        if let Some(apiversion) = kubeconfig.get(&Yaml::String("apiVersion".to_owned())) {
            if let Some(kind) = kubeconfig.get(&Yaml::String("kind".to_owned())) {
                if apiversion.as_str() == Some("v1") && kind.as_str() == Some("Config") {
                    return true;
                }
            }
        }
    }
    false
}

fn read_context(ctx: &Yaml) -> anyhow::Result<(&Yaml, &Hash)> {
    let ctxdata = ctx.as_hash().ok_or(anyhow!("Context not a hash"))?;
    let ctxname = ctxdata
        .get(&Yaml::String("name".to_owned()))
        .ok_or(anyhow!("context has no name?"))?;
    let context = ctxdata
        .get(&Yaml::String("context".to_owned()))
        .ok_or(anyhow!("Context has no field 'context'"))?
        .as_hash()
        .ok_or(anyhow!("Field 'context' not a hash"))?;
    Ok((ctxname, context))
}

pub struct Kubeconfig {
    config: Hash,
}

impl Kubeconfig {
    fn new() -> Self {
        let mut k = Self {
            config: Hash::new(),
        };
        k.config.insert(
            Yaml::String("apiVersion".to_owned()),
            Yaml::String("v1".to_owned()),
        );
        k.config.insert(
            Yaml::String("kind".to_owned()),
            Yaml::String("Config".to_owned()),
        );
        k.config
            .insert(Yaml::String("clusters".to_owned()), Yaml::Array(vec![]));
        k.config
            .insert(Yaml::String("contexts".to_owned()), Yaml::Array(vec![]));
        k.config
            .insert(Yaml::String("users".to_owned()), Yaml::Array(vec![]));
        k
    }

    fn add_context(
        &mut self,
        ctxname: &Yaml,
        ctx: &Yaml,
        cluster: &Yaml,
        user: &Yaml,
        current: bool,
    ) -> anyhow::Result<()> {
        let contexts = self.config[&Yaml::String("contexts".to_owned())]
            .as_mut_vec()
            .ok_or(anyhow!("contexts not vec?"))?;
        if contexts.contains(ctx) {
            return Err(anyhow!("Kubeconfig already contains context {ctx:?}"));
        }
        contexts.push(ctx.clone());
        let clusters = self.config[&Yaml::String("clusters".to_owned())]
            .as_mut_vec()
            .ok_or(anyhow!("clusters not vec?"))?;
        if !clusters.contains(cluster) {
            clusters.push(cluster.clone());
        }
        let users = self.config[&Yaml::String("users".to_owned())]
            .as_mut_vec()
            .ok_or(anyhow!("users not vec?"))?;
        if !users.contains(user) {
            users.push(user.clone());
        }

        if current {
            self.config
                .insert(Yaml::String("current-context".to_owned()), ctxname.clone());
        }

        Ok(())
    }

    pub fn write(&self, emitter: &mut YamlEmitter) -> anyhow::Result<()> {
        emitter
            .dump(&Yaml::Hash(self.config.clone()))
            .map_err(|e| anyhow!("{e}"))
    }
}

pub fn split_into_contexts(
    kubeconfig: &Yaml,
    output_file_pattern: &str,
) -> anyhow::Result<HashMap<String, Kubeconfig>> {
    let mut res = HashMap::new();

    let data = kubeconfig.as_hash().ok_or(anyhow!("Not a kubeconfig?"))?;
    let contexts = read_list(data, "contexts")?;
    let clusters = read_list(data, "clusters")?;
    let users = read_list(data, "users")?;

    for ctx in contexts {
        let (ctxname, ctxdata) = read_context(ctx)?;
        let cluster_name = read_string(ctxdata, "cluster")?;
        let cluster = find_entry(clusters, cluster_name)?.ok_or(anyhow!(
            "Cluster '{cluster_name}' not present in kubeconfig"
        ))?;
        let user_name = read_string(ctxdata, "user")?;
        let user = find_entry(users, user_name)?
            .ok_or(anyhow!("User '{user_name}' not present in kubeconfig"))?;
        let mut kubeconfig = Kubeconfig::new();
        kubeconfig.add_context(ctxname, ctx, cluster, user, true)?;
        let namespace = ctxdata
            .get(&Yaml::String("namespace".to_owned()))
            .ok_or(anyhow!("no namespace in context"))?
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
