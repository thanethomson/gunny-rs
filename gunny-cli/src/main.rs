use std::{
    error::Error,
    path::{Path, PathBuf},
};

use clap::{ArgEnum, Parser};
use gunny::Context;
use log::{debug, info};

#[derive(Parser, Debug)]
#[clap(name = "gunny", about, version)]
struct Args {
    /// Set output logging verbosity.
    #[clap(arg_enum, short, long, default_value = "info")]
    verbosity: Verbosity,

    /// Optional path to a configuration file. Any configuration provided by
    /// this file will automatically be available via a `config` variable in
    /// each view. Can be provided in either YAML or JSON format.
    #[clap(short, long, default_value = "config.json")]
    config: PathBuf,

    /// The path relative to which all output files will be written.
    #[clap(short, long, default_value = ".")]
    output_path: PathBuf,

    /// Which view(s) to render.
    views: Vec<String>,
}

#[derive(Debug, Clone, Copy, ArgEnum)]
enum Verbosity {
    Info,
    Debug,
    Trace,
}

impl From<Verbosity> for log::Level {
    fn from(v: Verbosity) -> Self {
        match v {
            Verbosity::Info => Self::Info,
            Verbosity::Debug => Self::Debug,
            Verbosity::Trace => Self::Trace,
        }
    }
}

fn main() {
    let args = Args::parse();
    simple_logger::init_with_level(args.verbosity.into()).unwrap();

    let views = if args.views.is_empty() {
        vec!["*.js", "views/*.js"]
    } else {
        args.views.iter().map(AsRef::as_ref).collect::<Vec<&str>>()
    };

    if let Err(e) = render_views(&args.config, &args.output_path, &views) {
        log::error!("Failed: {}", e);
    }
}

fn render_views(config: &Path, output_path: &Path, views: &[&str]) -> Result<(), Box<dyn Error>> {
    let mut ctx = Context::new(config, output_path)?;
    let _ = ctx.load_views(views)?;
    debug!("Rendering views...");
    let output_count = ctx.render_all()?;
    info!(
        "Wrote {} output file{}",
        output_count,
        if output_count == 1 { "" } else { "s" }
    );
    Ok(())
}
