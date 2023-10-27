#![feature(async_closure)]

extern crate spider;

use spider::tokio;
use spider::website::Website;
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
        #[arg(short, long, env = "HOSTNAME", default_value_t = String::from("localhost"))]
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
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crawl_route = path!("crawl" / String).map(
        async move |url: String| {
            crawl_website(&url).await;
            format!("Scraped {}!", url)
        }
    );

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::Serve {hostname,port} => {
            let addr: std::net::SocketAddr = format!("{}:{}", hostname, port).parse().unwrap();
            warp::serve(crawl_route).run(addr).await;
            println!("Listening on {}:{}", hostname, port);
        },
        Commands::Exec { url} => {
            crawl_website(&url).await;
        }
    }

    Ok(())
}

async fn crawl_website(domain: &str) {
    let mut website = Website::new(domain);

    website
        .with_respect_robots_txt(true)
        .with_subdomains(true)
        .with_tld(false)
        .with_delay(0)
        .with_request_timeout(None)
        .with_http2_prior_knowledge(false)
        .with_user_agent(Some("myapp/version".into()))
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
}
