// C:\centric\signal\src\main.rs

use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc; // <-- YENİ: Arc'ı import ediyoruz
use tokio::net::UdpSocket;

// gRPC istemcimiz için gerekli importlar
pub mod voipcore {
    tonic::include_proto!("voipcore");
}
use voipcore::voip_core_client::VoipCoreClient;
use voipcore::CallRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listen_addr = "0.0.0.0:5060";
    // Soketi oluşturup hemen Arc içine sarıyoruz.
    let sock = Arc::new(UdpSocket::bind(listen_addr).await?);
    println!(">> SIP Sunucumuz başlatıldı, dinleniyor: {}", listen_addr);

    let mut buf = [0; 65535];

    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        
        // Arc'ı klonlamak çok hafiftir, sadece referans sayacını artırır.
        let sock_clone = Arc::clone(&sock);
        let request_bytes = buf[..len].to_vec();
        
        tokio::spawn(async move {
            if let Err(e) = handle_sip_request(&request_bytes, sock_clone, addr).await {
                eprintln!("[HATA] İstek işlenirken hata oluştu: {:?}", e);
            }
        });
    }
}

// Fonksiyon imzasını Arc<UdpSocket> alacak şekilde güncelliyoruz.
async fn handle_sip_request(
    request_bytes: &[u8],
    sock: Arc<UdpSocket>,
    addr: std::net::SocketAddr,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let request_str = std::str::from_utf8(request_bytes)?;

    if request_str.starts_with("INVITE") {
        if let Some(headers) = parse_headers(request_str) {
            let trying_response = create_response("100 Trying", &headers);
            sock.send_to(trying_response.as_bytes(), addr).await?;
            println!("<<< '100 Trying' gönderildi.");

            println!(">>> Core servisine yönlendirme isteği gönderiliyor...");
            match route_call_with_core(&headers).await {
                Ok(core_response) => {
                    println!("<<< Core'dan yanıt alındı: {:?}", core_response);
                    // BİR SONRAKİ ADIMDA BURADA '180 Ringing' ve '200 OK' GÖNDERECEĞİZ
                },
                Err(e) => {
                    eprintln!("[HATA] Core servisine ulaşılamadı: {}", e);
                }
            }
        }
    }
    Ok(())
}

async fn route_call_with_core(headers: &HashMap<String, String>) -> Result<voipcore::CallResponse, Box<dyn Error + Send + Sync>> {
    let mut client = VoipCoreClient::connect("http://127.0.0.1:50051").await?;
    let request = tonic::Request::new(CallRequest {
        from: headers.get("From").cloned().unwrap_or_default(),
        to: headers.get("To").cloned().unwrap_or_default(),
    });
    let response = client.route_call(request).await?;
    Ok(response.into_inner())
}

fn create_response(status_line: &str, headers: &HashMap<String, String>) -> String {
    format!(
        "SIP/2.0 {}\r\nVia: {}\r\nFrom: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {}\r\nContent-Length: 0\r\n\r\n",
        status_line,
        headers.get("Via").unwrap_or(&String::new()),
        headers.get("From").unwrap_or(&String::new()),
        headers.get("To").unwrap_or(&String::new()),
        headers.get("Call-ID").unwrap_or(&String::new()),
        headers.get("CSeq").unwrap_or(&String::new())
    )
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