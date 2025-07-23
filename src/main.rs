// C:\centric\signal\src\main.rs

// Tokio'nun UDP soketi ve asenkron özellikleri için gerekli importlar
use tokio::net::UdpSocket;
use std::io;
use std::str;

// Tokio'nun asenkron main fonksiyonu
#[tokio::main]
async fn main() -> io::Result<()> {
    // Dinleyeceğimiz adres ve port. 0.0.0.0 tüm ağ arayüzlerini dinler.
    let listen_addr = "0.0.0.0:5060";

    // Belirtilen adreste bir UDP soketi oluşturuyoruz.
    // `?` operatörü, bir hata olursa programı durdurur ve hatayı döndürür.
    let sock = UdpSocket::bind(listen_addr).await?;
    
    println!("SIP Sunucusu başlatıldı, dinleniyor: {}", listen_addr);

    // Gelen veriyi tutmak için bir tampon (buffer) oluşturuyoruz.
    // 65535, bir UDP paketinin alabileceği maksimum boyuttur.
    let mut buf = [0; 65535];

    // Sonsuz bir döngü başlatarak sürekli olarak gelen paketleri bekliyoruz.
    loop {
        // `sock.recv_from` fonksiyonu bir paket gelene kadar bekler.
        // Gelen paketin boyutunu ve kimden geldiğini (kaynak adresi) döndürür.
        let (len, addr) = sock.recv_from(&mut buf).await?;

        println!("\n--- Yeni Paket Alındı ---");
        println!("Kimden: {}", addr);
        println!("Boyut: {} bytes", len);

        // Gelen byte dizisini (buf) bir string'e dönüştürmeye çalışıyoruz.
        // SIP mesajları metin tabanlı olduğu için bu genellikle başarılı olur.
        match str::from_utf8(&buf[..len]) {
            Ok(message) => {
                println!("--- Mesaj İçeriği ---");
                println!("{}", message);
            },
            Err(e) => {
                eprintln!("HATA: Gelen veri UTF-8 formatında değil: {}", e);
            }
        }
        println!("------------------------\n");
    }
}