# Signal Servisi

`signal`, Centiric platformunun SIP/RTP sinyal sunucusudur. Rust dilinde yazılmıştır ve yüksek performanslı, güvenli bir "sınır muhafızı" olarak görev yapar.

## Hızlı Başlangıç (Geliştirme Ortamı)

Bu servisi yerel makinenizde çalıştırmak için aşağıdaki adımları izleyin.

### Gereksinimler
-   [Rust Toolchain](https://rustup.rs/)
-   [Protobuf Compiler](https://grpc.io/docs/protoc-installation/)
-   C++ Derleme Araçları ([Linux](https://packages.ubuntu.com/search?keywords=build-essential) / [Windows](https://visualstudio.microsoft.com/visual-cpp-build-tools/))

### Kurulum ve Çalıştırma

1.  **Repoyu Klonlama:**
    ```bash
    git clone https://github.com/Centiric/signal.git
    cd signal
    ```

2.  **Derleme ve Çalıştırma:**
    `cargo`, `.proto` dosyalarını derleme dahil tüm adımları otomatik olarak yönetir.
    ```bash
    cargo run
    ```
    Başarılı bir başlangıçtan sonra terminalde `SIP Sunucusu başlatıldı, dinleniyor: 0.0.0.0:5060` mesajını göreceksiniz.

### Test Etme

1.  **`core` Servisinin Çalıştığından Emin Olun:** `signal`'ın iletişim kurabilmesi için `core` servisinin `50051` portunda çalışıyor olması gerekir.
2.  **SIP İsteği Gönderme:** Bir Softphone uygulaması (örn: Zoiper) kullanarak `herhangi-bir-sey@localhost` adresine bir arama yapın. Terminalde gelen `INVITE` paketini görmelisiniz.
