use dns_lookup::lookup_host;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use std::error::Error;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Instant;

pub mod types;

fn make_tls_stream<C>(
    url: &str,
    tcp_stream: C,
) -> Result<StreamOwned<ClientConnection, C>, Box<dyn Error>>
where
    C: Read + Write,
{
    // TLS Configuration and Handshake
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let conn = ClientConnection::new(
        std::sync::Arc::new(config),
        url.to_owned().try_into().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "invalid dns name")
        })?,
    )?;
    let mut tls_stream = StreamOwned::new(conn, tcp_stream);
    // Force TLS handshake by attempting a write
    while tls_stream.conn.is_handshaking() {
        tls_stream.conn.complete_io(&mut tls_stream.sock)?;
    }
    Ok(tls_stream)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args().nth(1).expect("Usage: program <domain>");
    let port = 443;

    // DNS Resolution timing
    let dns_start = Instant::now();
    let ips = lookup_host(&url)?;
    let ip = ips.first().expect("no IP addresses returned");
    let dns_duration = dns_start.elapsed();
    let dns_start2 = Instant::now();
    let ips = lookup_host(&url)?;
    let _ip = ips.first().expect("no IP addresses returned");
    let dns_duration2 = dns_start2.elapsed();

    // TCP Connection timing
    let tcp_start = Instant::now();
    let tcp_stream = TcpStream::connect((*ip, port))?;
    let tcp_duration = tcp_start.elapsed();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");
    let tls_start = Instant::now();
    let mut tls_stream = make_tls_stream(&url, tcp_stream)?;
    let tls_duration = tls_start.elapsed();

    // HTTP GET Request timing
    let request = format!(
        "GET / HTTP/1.1\r\n\
         Host: {}\r\n\
         Connection: close\r\n\
         \r\n",
        &url
    );

    let get_start = Instant::now();
    tls_stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    tls_stream.read_to_end(&mut response)?;
    let get_duration = get_start.elapsed();

    let total_duration = dns_duration2 + tcp_duration + tls_duration + get_duration;

    // Parse headers from response
    let response_str = String::from_utf8_lossy(&response);
    let mut headers = response_str
        .split("\r\n\r\n")
        .next()
        .unwrap_or("")
        .split("\r\n");

    // Skip the status line
    headers.next();

    let mut response_region: Option<String> = None;
    let mut ip_address: Option<String> = None;
    for header in headers {
        if header.to_lowercase().contains("region") {
            if let Some((_, value)) = header.split_once(':') {
                response_region = Some(value.trim().into());
            }
        }
        if header.to_lowercase().contains("ip_address") {
            if let Some((_, value)) = header.split_once(':') {
                ip_address = Some(value.trim().into());
            }
        }
    }

    let report = types::Report {
        client_region: std::env::var("FLY_REGION").unwrap_or("IDK".into()),
        dns_duration,
        dns_duration2,
        tcp_duration,
        tls_duration,
        get_duration,
        total_duration,
        response_region: response_region.unwrap_or("IDK".into()),
        ip_address: ip_address.unwrap_or("IDK".into()),
    };
    println!("{report:#?}");
    // Send report to collection endpoint
    let collection_url =
        std::env::var("COLLECTION_URL").unwrap_or_else(|_| "https://httpbin.org/post".to_string());

    println!("Sending report to {}", collection_url);

    let response = ureq::post(&collection_url)
        .set("Content-Type", "application/json")
        .set("Authorization", "Bearer asdasd")
        .send_json(&report)
        .expect("Failed to POST");
    println!("{}", response.into_string().unwrap());
    Ok(())
}
