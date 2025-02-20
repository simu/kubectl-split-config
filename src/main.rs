use std::io::Write;
use std::path::PathBuf;

use anyhow::anyhow;
use clap::Parser;
use saphyr::{Yaml, YamlEmitter};

mod kubeconfig;
mod yaml;

use kubeconfig::{is_kubeconfig, split_into_contexts};

#[derive(Parser, Debug)]
#[clap(name = "kubectl_split-config")]
#[clap(bin_name = "kubectl_split-config")]
#[clap(version)]
struct Cli {
    /// Kubeconfig file to split
    file: PathBuf,
    /// Output kubeconfig filename pattern.
    ///
    /// Supported format specifiers: CLUSTER, NAMESPACE, USER. These are replaced by the context's
    /// cluster, namespace and user values. Can be a relative or absolute path.
    ///
    /// If the pattern doesn't generate unique names for each context, the tool will exit without
    /// writing any files.
    #[arg(short, long, default_value = "CLUSTER-USER-NAMESPACE.kubeconfig")]
    output_file_pattern: String,
    /// Skip contexts which contain this string in their name field when splitting the input file.
    #[arg(short, long, default_value = None)]
    skip_string: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    if !args.file.is_file() {
        return Err(anyhow!("File {} doesn't exist", args.file.display()));
    }

    let fdata = std::fs::read_to_string(&args.file)?;
    let fcontents = Yaml::load_from_str(&fdata)?;
    if fcontents.len() > 1 {
        return Err(anyhow!(
            "File {} contains multiple YAML documents",
            args.file.display()
        ));
    }
    let kubeconfig = &fcontents[0];
    if !is_kubeconfig(kubeconfig) {
        return Err(anyhow!(
            "File {} isn't a kubeconfig file",
            args.file.display()
        ));
    }
    println!(
        "Generating files for each context in {}",
        args.file.display()
    );

    let outparts = args
        .output_file_pattern
        .split(std::path::MAIN_SEPARATOR)
        .collect::<Vec<_>>();
    let outpattern = outparts[outparts.len() - 1];
    let outpath = outparts[..outparts.len() - 1].join(std::path::MAIN_SEPARATOR_STR);

    let context_configs = split_into_contexts(kubeconfig, outpattern, &args.skip_string)?;

    for (fname, cfg) in context_configs {
        let mut s = String::new();
        let mut e = YamlEmitter::new(&mut s);
        e.compact(false);

        cfg.write(&mut e)?;

        let mut fp = PathBuf::from(&outpath);
        fp.push(fname);
        println!("Writing {}", fp.display());
        let mut f = std::fs::File::create(fp)?;
        f.write_all(s.as_bytes())?;
        f.write("\n".as_bytes())?;
    }

    Ok(())
}
