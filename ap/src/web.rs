use crate::routing::{HttpMethod, RequestType};
use embassy_net::{IpListenEndpoint, Stack, tcp::TcpSocket};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write;
use esp_println::{self as _, println};
const INDEX_HTML: &str = include_str!(concat!(env!("OUT_DIR"), "/generated_html.html"));

#[embassy_executor::task]
pub async fn handle_requests(stack: Stack<'static>, board: crate::util::Board) {
    #[allow(non_snake_case)]
    let PORT = option_env!("PORT")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    let mut led = board.led;
    let mut espnow = board.espnow;
    espnow.set_channel(2).unwrap();

    let mut rx_buffer = [0_u8; 1536];
    let mut tx_buffer = [0_u8; 1536];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(embassy_time::Duration::from_secs(30)));
    loop {
        let r = socket
            .accept(IpListenEndpoint {
                addr: None,
                port: PORT,
            })
            .await;
        if let Err(e) = r {
            println!("Connect error: {:?}", e);
            continue;
        }
        loop {
            match crate::routing::read_socket(&mut socket).await {
                Err(e) => {
                    println!("[ERROR] {:?}", e);
                    break;
                }
                Ok(method) => match method {
                    RequestType::Upgrade(key) => {
                        match crate::routing::approve_web_socket(&mut socket, key).await {
                            Err(e) => println!("socket approval err: {:?}", e),
                            Ok(()) => {
                                println!("SOCKET APPROVED!");
                                crate::routing::handle_ws(&mut socket, &mut led, &mut espnow).await;
                                break;
                            }
                        }
                    }
                    RequestType::Standard(HttpMethod::Get) => {
                        let r = socket.write_all(INDEX_HTML.as_bytes()).await;
                        if let Err(e) = r {
                            println!("write error: {:?}", e);
                            let r = socket.flush().await;
                            if let Err(e) = r {
                                println!("flush error: {:?}", e);
                            }
                        }
                        break;
                    }
                    other => {
                        println!("unmapped request: {:?}", other);
                    }
                },
            };
        }
        socket.close();
        Timer::after(Duration::from_millis(500)).await;
        socket.abort();
    }
}
