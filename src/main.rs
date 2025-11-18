mod structures;

use std::{fs::File, io::BufReader, sync::RwLock, time::Duration};

use actix_web::{get, middleware::Logger, rt::time::sleep, web, App, HttpResponse, HttpServer};
use askama::Template;
use clap::{command, Parser};
use futures::{stream, StreamExt, TryStreamExt};
use log::{debug, error, info, warn};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use structures::shoutrrr::notify;

use crate::structures::{
    errors::UptimersError,
    model::{SiteFactModel, SiteFullModel, SiteModel, SiteStatModel},
};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    sites: Vec<SiteModel>,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Address to listen on
    #[arg(long, env, default_value = "0.0.0.0")]
    bind_address: String,

    /// Port to listen on
    #[arg(long, env, default_value_t = 8080)]
    bind_port: u16,

    /// Postgres username
    #[arg(long, env, default_value = "uptimers")]
    postgres_username: String,

    /// Postgres password
    #[arg(long, env, default_value = "password")]
    postgres_password: String,

    /// Postgres host
    #[arg(long, env, default_value = "localhost")]
    postgres_host: String,

    /// Postgres DB name
    #[arg(long, env, default_value = "uptimers")]
    postgres_db: String,

    /// Shoutrrr URL
    #[arg(long, env)]
    shoutrrr_url: Option<String>,

    /// path to config file
    #[arg(long, env, default_value = "./config.yaml")]
    config_path: String,
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    sites: Vec<SiteStatModel>,
}

#[get("/")]
async fn index_handler(page: web::Data<RwLock<String>>) -> Result<HttpResponse, UptimersError> {
    debug!("Acquiring read lock on the shared pre-generated page");
    let page = (*page.read().unwrap()).clone();
    debug!("Acquired read lock");
    Ok(HttpResponse::Ok().content_type("text/html").body(page))
}

async fn garbage_collect(pool: &PgPool) -> Result<(), UptimersError> {
    let result = sqlx::query!("DELETE FROM site_fact WHERE tstamp < now() - interval '31 day'")
        .execute(pool)
        .await?;
    debug!("Garbage collected {} rows", result.rows_affected());
    Ok(())
}

async fn connect_site(
    client: &Client,
    site: &SiteModel,
    pool: &PgPool,
    shoutrrr_url: &Option<String>,
) -> Result<(StatusCode, bool), UptimersError> {
    let mut status_code = StatusCode::BAD_GATEWAY;
    let max_attempts = 5;
    let url = site.site.as_str();

    for attempt in 0..max_attempts {
        match client
            .get(url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                info!("Connected to {}: {}", url, status);
                status_code = status;
                break;
            }
            Err(e) => {
                warn!(
                    "Failed to connect to {}: {} (attempt {} out of {})",
                    url,
                    e,
                    attempt + 1,
                    max_attempts
                );

                // Sleep and try again if we're not on the last attempt
                if attempt + 1 < max_attempts {
                    sleep(Duration::from_secs(attempt + 1)).await;
                }
            }
        };
    }

    let site_fact = sqlx::query_as!(
        SiteFactModel,
        r#"SELECT
            site,
            tstamp,
            success,
            status_code
        FROM site_fact s1
        WHERE
            tstamp = (SELECT MAX(tstamp) FROM site_fact s2 WHERE s1.site = s2.site)
        AND site = $1;"#,
        site.site
    )
    .fetch_optional(pool)
    .await?;
    let previous_success = site_fact.map_or_else(|| true, |site| site.success);
    let current_success = status_code.is_success()
        || site
            .success_codes
            .as_ref()
            .unwrap_or(&Vec::new())
            .contains(&status_code.as_u16());
    debug!(
        "Previous success: {}, Current success: {}",
        previous_success, current_success
    );

    if let Some(shoutrrr_url) = shoutrrr_url {
        if previous_success != current_success {
            let msg = if current_success {
                format!("🟢 {} up: {}", site.name, status_code)
            } else {
                format!("🔴 {} down: {}", site.name, status_code)
            };

            if let Err(e) = notify(shoutrrr_url, msg) {
                error!("Failed to send notification to shoutrrr: {}", e);
            }
        }
    }
    Ok((status_code, current_success))
}

async fn connect_sites(
    sites: &Vec<SiteModel>,
    client: &Client,
    pool: &PgPool,
    shoutrrr_url: &Option<String>,
) -> Result<(), UptimersError> {
    // Truncate the current timestamp to minute accuracy
    let now = time::OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .unwrap()
        .replace_second(0)
        .unwrap();
    debug!("Connecting to sites with timestamp {}", now);

    // Connect to all user-supplied URLs and write results into the database
    stream::iter(sites)
        // Attempt to connect to all URLs
        .map(|site| async move {
            let url = &site.site;
            debug!("Attempting to connect to {}", url);
            let (status_code, success) = connect_site(client, site, pool, shoutrrr_url).await?;
            Ok::<SiteFullModel, UptimersError>(SiteFullModel {
                site: url.to_string(),
                name: site.name.clone(),
                tstamp: now,
                success,
                status_code: status_code.as_u16() as i16,
            })
        })
        // Run 5 parallel at a time
        .buffer_unordered(5)
        // Try to update the DB with the connection data
        .try_for_each_concurrent(5, |site| async move {
            debug!("Writing timestamp data into DB: {}", site.site);
            sqlx::query!(
                r#"INSERT INTO site_fact(
                        site,
                        tstamp,
                        success,
                        status_code
                    )
                    VALUES ($1, $2, $3, $4)
                    ON CONFLICT (site, tstamp)
                    DO NOTHING"#,
                site.site,
                site.tstamp,
                site.success,
                site.status_code,
            )
            .execute(pool)
            .await?;

            debug!("Writing site data into DB: {}", site.site);
            sqlx::query!(
                r#"INSERT INTO site(
                        site,
                        name
                    )
                    VALUES ($1, $2)
                    ON CONFLICT (site)
                    DO UPDATE
                        SET site = $1, name =$2"#,
                site.site,
                site.name,
            )
            .execute(pool)
            .await?;

            Ok(())
        })
        .await
}

async fn render_page(
    sites: &Vec<SiteModel>,
    page: &RwLock<String>,
    pool: &PgPool,
) -> Result<(), UptimersError> {
    // Select the most recent timestamps and 24 hours average uptime from each site
    let status = sqlx::query_as!(
        SiteStatModel,
        r#"SELECT
            t1.site,
            t3.name,
            t1.tstamp,
            t1.success,
            t1.status_code,
            t2.avg
        FROM (
            SELECT
                site,
                tstamp,
                success,
                status_code
            FROM site_fact s1
            WHERE
                tstamp = (SELECT MAX(tstamp) FROM site_fact s2 WHERE s1.site = s2.site)
            AND
                site = ANY($1)
            ORDER BY site, tstamp
        ) t1
        INNER JOIN (
            SELECT
                site,
                AVG(success::int::float)
            FROM
                site_fact WHERE tstamp >= (NOW() - INTERVAL '1 day')
            GROUP BY
                site
        ) t2
        ON
            t1.site = t2.site
        INNER JOIN (
            SELECT
                site,
                name
            FROM
                site
        ) t3
        ON
            t1.site = t3.site
        ORDER BY
            t3.name;"#,
        &sites
            .into_iter()
            .map(|site| site.site.clone())
            .collect::<Vec<_>>(),
    )
    .fetch_all(pool)
    .await?;

    debug!("{:?}", status);
    let index = IndexTemplate { sites: status };
    // Update the page string reference shared with the index_handler thread
    {
        debug!("Acquiring write lock to update the pre-generated status page");
        let mut p = page.write().unwrap();
        *p = index.render()?;
    }
    debug!("Released write lock");

    Ok(())
}

#[actix_web::main]
async fn main() -> Result<(), UptimersError> {
    let args = Args::parse();
    info!("Started with args: {:?}", args);

    info!("Reading config from {}", args.config_path);
    let config: Config = serde_yaml::from_reader(BufReader::new(File::open(&args.config_path)?))?;
    debug!("Config: {:?}", config);

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // web::Data and PgPool use Arc internally, so clones are cheap and usage is threadsafe
    let sites = web::Data::new(config.sites);
    // Pass around a pre-rendered template rather than rendering on every request
    let page = web::Data::new(RwLock::new(
        "Initial launch, please wait a few seconds...".to_string(),
    ));
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&format!(
            "postgres://{}:{}@{}/{}",
            args.postgres_username, args.postgres_password, args.postgres_host, args.postgres_db,
        ))
        .await?;
    sqlx::migrate!().run(&pool).await?;

    // Spawn background loop to check on all user-supplied URLs every minute
    actix_web::rt::spawn({
        let sites = sites.clone();
        let page = page.clone();
        let client = Client::new();
        let pool = pool.clone();
        async move {
            loop {
                if let Err(e) = connect_sites(&sites, &client, &pool, &args.shoutrrr_url).await {
                    error!("Failed to get site status: {}", e);
                };

                if let Err(e) = render_page(&sites, &page, &pool).await {
                    error!("Failed to render status page: {}", e);
                };

                if let Err(e) = garbage_collect(&pool).await {
                    error!("Failed to garbage collect: {}", e);
                };

                sleep(Duration::from_secs(60)).await;
            }
        }
    });

    Ok(HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(page.clone())
            .service(index_handler)
    })
    .bind((args.bind_address, args.bind_port))?
    .run()
    .await?)
}
