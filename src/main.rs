use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UdpSocket;
use rand::Rng;
use config::{Config, File};
use serde::Deserialize;
use tracing::{info, error, debug, instrument, Level};
use tracing_subscriber::FmtSubscriber;

// gRPC Bölümü - Derlenen proto kodunu dahil et
pub mod voipcore {
    tonic::include_proto!("voipcore");
}
use voipcore::voip_core_client::VoipCoreClient;
use voipcore::CallRequest;

// Konfigürasyon Yapıları
#[derive(Debug, Deserialize)]
struct SipConfig {
    host: String,
    port: u16,
    public_ip: String,
}
#[derive(Debug, Deserialize)]
struct CoreConfig {
    address: String,
}
#[derive(Debug, Deserialize)]
struct Settings {
    sip: SipConfig,
    core: CoreConfig,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Loglama kurulumu
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Konfigürasyon yükleme
    let settings = Config::builder()
        .add_source(File::with_name("config/default"))
        .build()?
        .try_deserialize::<Settings>()?;
    info!(config = ?settings, "Konfigürasyon başarıyla yüklendi");

    let listen_addr = format!("{}:{}", settings.sip.host, settings.sip.port);
    let sock = Arc::new(UdpSocket::bind(&listen_addr).await?);
    info!(address = %listen_addr, "SIP Sunucumuz başlatıldı");

    let settings = Arc::new(settings);
    let mut buf = [0; 65535];

    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let sock_clone = Arc::clone(&sock);
        let settings_clone = Arc::clone(&settings);
        let request_bytes = buf[..len].to_vec();
        
        tokio::spawn(async move {
            if let Err(e) = handle_sip_request(&request_bytes, sock_clone, addr, settings_clone).await {
                error!(error = %e, "İstek işlenirken hata oluştu");
            }
        });
    }
}

#[instrument(skip(request_bytes, sock, settings))]
async fn handle_sip_request(
    request_bytes: &[u8],
    sock: Arc<UdpSocket>,
    addr: std::net::SocketAddr,
    settings: Arc<Settings>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request_str = std::str::from_utf8(request_bytes)?;

    if request_str.starts_with("INVITE") {
        debug!(from = %addr, "INVITE paketi alındı");
        if let Some(mut headers) = parse_complex_headers(request_str) {
            
            let trying_response = create_response("100 Trying", &headers, None, &settings.sip);
            sock.send_to(trying_response.as_bytes(), addr).await?;
            info!("'100 Trying' gönderildi.");

            match route_call_with_core(&headers, &settings.core.address).await {
                Ok(core_response) => {
                    info!(session_id = %core_response.session_id, rtp_port = core_response.rtp_port, "Core'dan yanıt alındı");
                    
                    if core_response.status == 0 {
                        let to_header = headers.get("To").cloned().unwrap_or_default();
                        let to_tag = format!(";tag={}", rand::thread_rng().gen::<u32>());
                        headers.insert("To".to_string(), format!("{}{}", to_header, to_tag));

                        let ringing_response = create_response("180 Ringing", &headers, None, &settings.sip);
                        sock.send_to(ringing_response.as_bytes(), addr).await?;
                        
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        let sdp_body = format!(
                            "v=0\r\no=- 0 0 IN IP4 {0}\r\ns=Centiric\r\nc=IN IP4 {0}\r\nt=0 0\r\nm=audio {1} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
                            settings.sip.public_ip, core_response.rtp_port
                        );

                        let ok_response = create_response("200 OK", &headers, Some(&sdp_body), &settings.sip);
                        sock.send_to(ok_response.as_bytes(), addr).await?;
                        info!(port = core_response.rtp_port, "Arama başarıyla cevaplandı!");
                    }
                },
                Err(e) => { error!(error = %e, "Core servisine ulaşılamadı"); }
            }
        }
    }
    Ok(())
}

#[instrument(skip(headers, core_address))]
async fn route_call_with_core(headers: &HashMap<String, String>, core_address: &str) -> Result<voipcore::CallResponse, Box<dyn std::error::Error + Send + Sync>> {
    debug!(address = %core_address, "Core servisine bağlanılıyor");
    let mut client = VoipCoreClient::connect(core_address.to_string()).await?;

    let request = tonic::Request::new(CallRequest {
        from: headers.get("From").cloned().unwrap_or_default(),
        to: headers.get("To").cloned().unwrap_or_default(),
    });

    let response = client.route_call(request).await?;
    Ok(response.into_inner())
}

// --- EKSİK FONKSİYONLAR EKLENDİ ---
/// Gelen SIP metnini ayrıştırır ve çoklu 'Via' ile 'Record-Route' başlıklarını doğru işler.
fn parse_complex_headers(request: &str) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();
    let mut via_headers = Vec::new();
    let mut record_route_headers = Vec::new();

    for line in request.lines() {
        if line.is_empty() { break; }

        if let Some((key, value)) = line.split_once(':') {
            let key_trimmed = key.trim();
            let value_trimmed = value.trim();

            match key_trimmed.to_lowercase().as_str() {
                "via" | "v" => via_headers.push(value_trimmed),
                "record-route" => record_route_headers.push(value_trimmed),
                _ => { headers.insert(key_trimmed.to_string(), value_trimmed.to_string()); }
            }
        }
    }

    if !via_headers.is_empty() {
        headers.insert("Via".to_string(), via_headers.join("\r\nVia: "));
        if !record_route_headers.is_empty() {
            headers.insert("Record-Route".to_string(), record_route_headers.join("\r\nRecord-Route: "));
        }
        Some(headers)
    } else {
        None
    }
}

/// Cevap oluştururken Record-Route'u da ekler.
fn create_response(status_line: &str, headers: &HashMap<String, String>, sdp: Option<&str>, sip_config: &SipConfig) -> String {
    let body = sdp.unwrap_or("");
    let content_length = body.len();

    let record_route_line = match headers.get("Record-Route") {
        Some(routes) => format!("Record-Route: {}\r\n", routes),
        None => String::new(),
    };
    
    // Contact başlığı için IP ve Port'u konfigürasyondan alıyoruz
    let contact_ip = &sip_config.public_ip;
    let contact_port = sip_config.port;

    format!(
        "SIP/2.0 {}\r\n\
         Via: {}\r\n\
         {}\
         From: {}\r\n\
         To: {}\r\n\
         Call-ID: {}\r\n\
         CSeq: {}\r\n\
         Contact: <sip:{}@{}:{}>\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: {}\r\n\r\n\
         {}",
        status_line,
        headers.get("Via").unwrap_or(&String::new()),
        record_route_line,
        headers.get("From").unwrap_or(&String::new()),
        headers.get("To").unwrap_or(&String::new()),
        headers.get("Call-ID").unwrap_or(&String::new()),
        headers.get("CSeq").unwrap_or(&String::new()),
        "signal", contact_ip, contact_port, // Contact başlığı için kullanıcı, ip ve port
        content_length,
        body
    )
}
// ------------------------------------