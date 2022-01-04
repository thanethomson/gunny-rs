use std::error::Error;

use clap::Parser;
use gunny::Context;

#[derive(Parser, Debug)]
#[clap(name = "gunny", about, version)]
struct Args {
    /// Increase output logging verbosity.
    #[clap(short, long)]
    verbose: bool,

    /// Which view(s) to render.
    views: Vec<String>,
}

fn main() {
    let args = Args::parse();
    simple_logger::init_with_level(if args.verbose {
        log::Level::Debug
    } else {
        log::Level::Info
    })
    .unwrap();

    let views = if args.views.is_empty() {
        vec!["*.js", "views/*.js"]
    } else {
        args.views.iter().map(AsRef::as_ref).collect::<Vec<&str>>()
    };

    match render_views(&views) {
        Ok(_) => log::info!("Success!"),
        Err(e) => log::error!("Failed: {}", e),
    }
}

fn render_views(views: &[&str]) -> Result<(), Box<dyn Error>> {
    let mut ctx = Context::default();
    let _ = ctx.load_views(views)?;
    ctx.render_all()?;
    Ok(())
}
