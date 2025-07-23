// C:\centric\signal\src\main.rs

pub mod voipcore {
    tonic::include_proto!("voipcore");
}

use voipcore::voip_core_client::VoipCoreClient;
use voipcore::CallRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Signal servisi başlatılıyor...");
    
    // GÜNCELLENMİŞ PORT: 50051
    let mut client = VoipCoreClient::connect("http://127.0.0.1:50051").await?;
    
    println!("Core servisine başarıyla bağlanıldı!");
    
    let request = tonic::Request::new(CallRequest {
        from: "sip:gercek_bir_numara@dis.dunya".to_string(),
        to: "sip:yonlendirilecek_kisi@sirket.ici".to_string(),
    });
    
    println!("'RouteCall' isteği gönderiliyor...");
    let response = client.route_call(request).await?;
    
    println!("\n--- Core'dan Gelen Yanıt ---");
    println!("{:?}", response.into_inner());
    println!("--------------------------");

    // DİKKAT: Sondaki fazladan tırnaklar kaldırıldı.
    Ok(())
}