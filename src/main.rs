#![feature(async_closure)]

extern crate spider;

use spider::tokio;
use spider::website::Website;
use warp::http::StatusCode;
use warp::path;
use warp::Filter;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version=env!("CARGO_PKG_VERSION"), about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// server
    Serve {
        /// Hostname of server.
        #[arg(long, env = "HOSTNAME", default_value_t = String::from("localhost"))]
        hostname: String,

        /// Port for server to listen on.
        #[arg(short, long, env = "PORT", default_value_t = 3030)]
        port: u16,
    },

    /// CLI
    Exec {
        /// URL to scrape.
        #[arg(short, long, env = "URL")]
        url: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let version = path!("api" / "version").map(|| {
        let version = std::collections::HashMap::from([("version", env!("CARGO_PKG_VERSION"))]);

        //let version: String = format!("{{\"version\":\"{}\"}}", env!("CARGO_PKG_VERSION"));
        warp::reply::json(&version)
    });
    let crawl_route = path!("api" / "crawl" / String).and_then(crawl_website);

    let cli = Cli::parse();

    match &cli.command {
        Commands::Serve { hostname, port } => {
            let addr: std::net::SocketAddr = format!(
                "{}:{}",
                if hostname == "localhost" {
                    "127.0.0.1"
                } else {
                    hostname
                },
                port
            )
            .parse()?;
            println!("Shall listen on {}", addr);
            warp::serve(version.or(crawl_route)).run(addr).await;
        }
        Commands::Exec { url } => {
            crawl_website(url).await?;
        }
    }

    Ok(())
}

async fn crawl_website(
    domain: impl Into<String>,
) -> Result<impl warp::Reply, std::convert::Infallible> {
    let mut website = Website::new(&domain.into());

    website
        .with_respect_robots_txt(true)
        .with_subdomains(true)
        .with_tld(false)
        .with_delay(0)
        .with_request_timeout(None)
        .with_http2_prior_knowledge(false)
        .with_user_agent(Some(&format!("{}/version", env!("CARGO_PKG_NAME"))))
        // requires the `budget` feature flag
        .with_budget(Some(spider::hashbrown::HashMap::from([
            ("*", 300),
            ("/licenses", 10),
        ])))
        // .with_on_link_find_callback(Some(|link, html| {
        //     println!("link target: {}", link.inner());
        //     (link, html)
        // }))
        .with_external_domains(Some(
            Vec::from(["https://creativecommons.org/licenses/by/3.0/"].map(|d| d.to_string()))
                .into_iter(),
        ))
        .with_headers(None)
        .with_blacklist_url(Some(Vec::from([
            "https://choosealicense.com/licenses/".into()
        ])))
        .with_proxies(None);
    website.crawl().await;

    for link in website.get_links() {
        println!("- {:?}", link.as_ref());
    }
    Ok(StatusCode::OK)
}
