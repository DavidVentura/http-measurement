use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use axum_extra::TypedHeader;
use headers::{authorization::Bearer, Authorization};
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::net::SocketAddr;

pub mod types;

#[derive(Debug, Serialize)]
struct ReportResponse {
    id: i64,
    message: String,
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    token: String,
}

async fn create_indices(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_reports_created_at 
        ON reports(created_at);"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_reports_client_region 
        ON reports(client_region);"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE INDEX IF NOT EXISTS idx_reports_response_region 
        ON reports(response_region);"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS reports (
            id                BIGSERIAL PRIMARY KEY,
            created_at        TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            client_region     TEXT NOT NULL,
            response_region   TEXT NOT NULL,
            ip_address        TEXT NOT NULL,
            dns_duration_ms    DOUBLE PRECISION NOT NULL,
            dns2_duration_ms   DOUBLE PRECISION NOT NULL,
            tcp_duration_ms    DOUBLE PRECISION NOT NULL,
            tls_duration_ms    DOUBLE PRECISION NOT NULL,
            get_duration_ms    DOUBLE PRECISION NOT NULL,
            total_duration_ms  DOUBLE PRECISION NOT NULL
        );"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct ReportId {
    id: i64,
}
async fn submit_report(
    TypedHeader(token): TypedHeader<Authorization<Bearer>>,
    State(state): State<AppState>,
    Json(report): Json<types::Report>,
) -> Result<Json<ReportResponse>, (axum::http::StatusCode, String)> {
    if token.token() != state.token {
        return Err((StatusCode::UNAUTHORIZED, "Bad token\n".into()));
    }
    println!("{report:?}");
    println!(
        "report dns in ms {}",
        report.dns_duration.as_secs_f64() * 1000.0
    );
    let record: ReportId = sqlx::query_as(
        r#"
        INSERT INTO reports 
        (client_region,
        response_region,
        ip_address,
        dns_duration_ms,
        dns2_duration_ms,
        tcp_duration_ms, 
        tls_duration_ms,
        get_duration_ms,
        total_duration_ms)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id
        "#,
    )
    .bind(report.client_region)
    .bind(report.response_region)
    .bind(report.ip_address)
    .bind(report.dns_duration.as_secs_f64() * 1000.0)
    .bind(report.dns_duration2.as_secs_f64() * 1000.0)
    .bind(report.tcp_duration.as_secs_f64() * 1000.0)
    .bind(report.tls_duration.as_secs_f64() * 1000.0)
    .bind(report.get_duration.as_secs_f64() * 1000.0)
    .bind(report.total_duration.as_secs_f64() * 1000.0)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
    })?;

    Ok(Json(ReportResponse {
        id: record.id,
        message: "Report created successfully".to_string(),
    }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get database URL from environment or use default
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let token = std::env::var("AUTH_TOKEN").expect("AUTH_TOKEN must be set");

    // Create connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Create table if it doesn't exist
    create_table(&pool).await?;
    create_indices(&pool).await?;

    // Build our application with routes
    let app = Router::new()
        .route("/report", post(submit_report))
        .with_state(AppState { pool, token });

    // Run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("listening on {}", addr);
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
