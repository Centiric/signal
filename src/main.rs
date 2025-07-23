use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::net::UdpSocket;
use rand::Rng; // Rastgele tag üretmek için

// --- BU BÖLÜM EKSİKTİ ---
// gRPC istemcimiz için gerekli importlar
pub mod voipcore {
    tonic::include_proto!("voipcore");
}
use voipcore::voip_core_client::VoipCoreClient;
use voipcore::CallRequest;
// -------------------------

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listen_addr = "0.0.0.0:5060";
    let sock = Arc::new(UdpSocket::bind(listen_addr).await?);
    println!(">> SIP Sunucumuz başlatıldı, dinleniyor: {}", listen_addr);

    let mut buf = [0; 65535];

    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let sock_clone = Arc::clone(&sock);
        let request_bytes = buf[..len].to_vec();
        
        tokio::spawn(async move {
            if let Err(e) = handle_sip_request(&request_bytes, sock_clone, addr).await {
                eprintln!("[HATA] İstek işlenirken hata oluştu: {:?}", e);
            }
        });
    }
}

async fn handle_sip_request(
    request_bytes: &[u8],
    sock: Arc<UdpSocket>,
    addr: std::net::SocketAddr,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let request_str = std::str::from_utf8(request_bytes)?;

    if request_str.starts_with("INVITE") {
        if let Some(mut headers) = parse_headers(request_str) {
            let trying_response = create_response("100 Trying", &headers, None);
            sock.send_to(trying_response.as_bytes(), addr).await?;
            println!("<<< '100 Trying' gönderildi.");

            match route_call_with_core(&headers).await {
                Ok(core_response) => {
                    println!("<<< Core'dan yanıt alındı: {:?}", core_response);
                    
                    if core_response.status == 0 {
                        println!(">>> Core aramayı onayladı. Cevaplar gönderiliyor...");

                        let to_header = headers.get("To").cloned().unwrap_or_default();
                        let to_tag = format!(";tag={}", generate_random_tag());
                        headers.insert("To".to_string(), format!("{}{}", to_header, to_tag));

                        let ringing_response = create_response("180 Ringing", &headers, None);
                        sock.send_to(ringing_response.as_bytes(), addr).await?;
                        
                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        // ÖNEMLİ: Kendi Public IP adresinizi buraya yazın.
                        let public_ip = "34.122.40.122"; 
                        let sdp_body = format!(
                            "v=0\r\n\
                             o=- 0 0 IN IP4 127.0.0.1\r\n\
                             s=Centiric\r\n\
                             c=IN IP4 {}\r\n\
                             t=0 0\r\n\
                             m=audio 9000 RTP/AVP 0\r\n\
                             a=rtpmap:0 PCMU/8000\r\n",
                             public_ip
                        );

                        let ok_response = create_response("200 OK", &headers, Some(&sdp_body));
                        sock.send_to(ok_response.as_bytes(), addr).await?;

                        println!("<<< Arama başarıyla cevaplandı!");
                    }
                },
                Err(e) => {
                    eprintln!("[HATA] Core servisine ulaşılamadı: {}", e);
                    let error_response = create_response("503 Service Unavailable", &headers, None);
                    sock.send_to(error_response.as_bytes(), addr).await?;
                }
            }
        }
    }
    Ok(())
}

// --- BU FONKSİYON EKSİKTİ ---
/// Core servisine gRPC isteği gönderen fonksiyon.
async fn route_call_with_core(headers: &HashMap<String, String>) -> Result<voipcore::CallResponse, Box<dyn Error + Send + Sync>> {
    let mut client = VoipCoreClient::connect("http://127.0.0.1:50051").await?;

    let request = tonic::Request::new(CallRequest {
        from: headers.get("From").cloned().unwrap_or_default(),
        to: headers.get("To").cloned().unwrap_or_default(),
    });

    let response = client.route_call(request).await?;
    Ok(response.into_inner())
}
// ------------------------------

fn create_response(status_line: &str, headers: &HashMap<String, String>, sdp: Option<&str>) -> String {
    let body = sdp.unwrap_or("");
    let content_length = body.len();

    format!(
        "SIP/2.0 {}\r\n\
         Via: {}\r\n\
         From: {}\r\n\
         To: {}\r\n\
         Call-ID: {}\r\n\
         CSeq: {}\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: {}\r\n\r\n\
         {}",
        status_line,
        headers.get("Via").unwrap_or(&String::new()),
        headers.get("From").unwrap_or(&String::new()),
        headers.get("To").unwrap_or(&String::new()),
        headers.get("Call-ID").unwrap_or(&String::new()),
        headers.get("CSeq").unwrap_or(&String::new()),
        content_length,
        body
    )
}

fn generate_random_tag() -> String {
    rand::thread_rng().gen::<u32>().to_string()
}

fn parse_headers(request: &str) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();
    for line in request.lines().filter(|l| !l.is_empty()) {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    if headers.contains_key("Via") { Some(headers) } else { None }
}