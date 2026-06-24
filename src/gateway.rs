use core::net::Ipv4Addr;
//Embassy
use embassy_executor::Spawner;
use embassy_net::{Ipv4Cidr, Runner, Stack, StackResources, StaticConfigV4};
use embassy_time::{Duration, Timer};
//Esp
use esp_backtrace as _;
use esp_hal::{peripherals::WIFI, rng::Rng};
use esp_println::{self as _, println};
use esp_radio::wifi::{Config, ControllerConfig, Interface, WifiController, ap::AccessPointConfig};
//custom
use crate::mk_static;

#[embassy_executor::task]
pub async fn run_dhcp(stack: Stack<'static>, gw_ip_addr: Ipv4Addr) {
    use core::net::{Ipv4Addr, SocketAddrV4};

    use edge_dhcp::{
        io::{self, DEFAULT_SERVER_PORT},
        server::{Server, ServerOptions},
    };
    use edge_nal::UdpBind;
    use edge_nal_embassy::{Udp, UdpBuffers};

    let ip = gw_ip_addr;

    let mut buf = [0u8; 1500];

    let mut gw_buf = [Ipv4Addr::UNSPECIFIED];

    let buffers = UdpBuffers::<3, 1024, 1024, 10>::new();
    let unbound_socket = Udp::new(stack, &buffers);
    let mut bound_socket = unbound_socket
        .bind(core::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            DEFAULT_SERVER_PORT,
        )))
        .await
        .unwrap();

    loop {
        _ = io::server::run(
            &mut Server::<_, 64>::new_with_et(ip),
            &ServerOptions::new(ip, Some(&mut gw_buf)),
            &mut bound_socket,
            &mut buf,
        )
        .await
        .inspect_err(|e| println!("DHCP server error: {:?}", e));
        Timer::after(Duration::from_millis(500)).await;
    }
}

#[embassy_executor::task]
pub async fn connection_log(controller: WifiController<'static>) {
    println!("start connection task");
    loop {
        let ev = controller
            .wait_for_access_point_connected_event_async()
            .await;
        match ev {
            Ok(esp_radio::wifi::AccessPointStationEventInfo::Connected(info)) => {
                println!("Station connected: {:?}", info);
            }
            Ok(esp_radio::wifi::AccessPointStationEventInfo::Disconnected(info)) => {
                println!("Station disconnected: {:?}", info);
            }
            _ => (),
        }
        Timer::after(Duration::from_millis(5000)).await
    }
}

#[embassy_executor::task]
pub async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}

pub async fn start_wifi(periferal: WIFI<'static>, rng: Rng, spawner: &Spawner) -> Stack<'static> {
    #[allow(non_snake_case)]
    let SSID: &str = option_env!("HOSTNAME").unwrap_or("Esphub");
    #[allow(non_snake_case)]
    let PASSWORD: &str = option_env!("PASSWORD").unwrap_or("password");
    #[allow(non_snake_case)]
    let PORT = option_env!("PORT")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);
    let ip_gateway = option_env!("GW_ADDR")
        .and_then(|s| s.parse::<Ipv4Addr>().ok())
        .unwrap_or(Ipv4Addr::new(10, 0, 2, 1));
    let ap_conf = Config::AccessPoint(
        AccessPointConfig::default()
            .with_ssid(SSID)
            .with_password(PASSWORD.into())
            .with_auth_method(esp_radio::wifi::AuthenticationMethod::Wpa2Personal),
    );

    let (wifi_ctl, wifi_if) = esp_radio::wifi::new(
        periferal,
        ControllerConfig::default().with_initial_config(ap_conf),
    )
    .expect("Failed to initialize Wi-Fi controller");
    // Gateway
    //
    let ap_device = wifi_if.access_point;
    let ap_config = embassy_net::Config::ipv4_static(StaticConfigV4 {
        address: Ipv4Cidr::new(ip_gateway, 24),
        gateway: Some(ip_gateway),
        dns_servers: Default::default(),
    });
    let seed = (rng.random() as u64) << 32 | rng.random() as u64;
    // Init network stacks
    let (stack, runner) = embassy_net::new(
        ap_device,
        ap_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );
    //spawn tasks
    spawner.spawn(connection_log(wifi_ctl).expect("Connection task spawning error"));
    spawner.spawn(net_task(runner).expect("Net task runner error"));
    spawner.spawn(run_dhcp(stack, ip_gateway).expect("DHCP task error"));
    stack.wait_config_up().await;
    println!("Ip gateway: http://{}:{}", ip_gateway, PORT);
    stack
}
