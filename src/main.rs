// C:\centric\signal\src\main.rs

use std::collections::HashMap;
use std::io;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listen_addr = "0.0.0.0:5060";
    let sock = UdpSocket::bind(listen_addr).await?;
    println!(">> Kendi SIP Sunucumuz başlatıldı, dinleniyor: {}", listen_addr);

    let mut buf = [0; 65535];

    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let request_str = match std::str::from_utf8(&buf[..len]) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("[HATA] Geçersiz UTF-8 verisi alındı.");
                continue;
            }
        };

        println!("\n--- Yeni Paket Alındı [Kimden: {}] ---", addr);
        // Gelen mesajın tamamını yazdırmayalım, çok uzun olabilir. Sadece ilk birkaç satırı.
        println!("{}\n...", request_str.lines().take(5).collect::<Vec<_>>().join("\n"));
        
        if request_str.starts_with("INVITE") {
            println!(">>> INVITE isteği algılandı. '100 Trying' hazırlanıyor...");

            if let Some(headers) = parse_headers(request_str) {
                // ---- HATAYI DÜZELTEN KOD ----
                // `get` ile değeri `&String` olarak alırız, `map` ile `&str`'e çeviririz.
                // Bulamazsak, varsayılan olarak boş bir `&str` kullanırız.
                let via = headers.get("Via").map(String::as_str).unwrap_or("");
                let from = headers.get("From").map(String::as_str).unwrap_or("");
                let to = headers.get("To").map(String::as_str).unwrap_or("");
                let call_id = headers.get("Call-ID").map(String::as_str).unwrap_or("");
                let cseq = headers.get("CSeq").map(String::as_str).unwrap_or("");
                // -----------------------------

                let response = format!(
                    "SIP/2.0 100 Trying\r\n\
                     Via: {}\r\n\
                     From: {}\r\n\
                     To: {}\r\n\
                     Call-ID: {}\r\n\
                     CSeq: {}\r\n\
                     Content-Length: 0\r\n\r\n",
                    via, from, to, call_id, cseq
                );

                if let Err(e) = sock.send_to(response.as_bytes(), addr).await {
                    eprintln!("[HATA] Cevap gönderilemedi: {}", e);
                } else {
                    println!("<<< '100 Trying' başarıyla gönderildi.");
                }
            }
        }
    }
}

fn parse_headers(request: &str) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();
    for line in request.lines().filter(|l| !l.is_empty()) {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    
    if headers.contains_key("Via") {
        Some(headers)
    } else {
        None
    }
}