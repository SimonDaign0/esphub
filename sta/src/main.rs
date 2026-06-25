#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
use esp_radio::wifi::{Config, ControllerConfig, sta::StationConfig};

use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use esp_println::{self as _, println};

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    // generator version: 1.3.0
    // generator parameters: --chip esp32s3 -o unstable-hal -o alloc -o wifi -o zed -o defmt -o esp-backtrace -o embassy

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 73744);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    info!("Embassy initialized!");
    let sta_conf = Config::Station(StationConfig::default().with_channel(2));
    let (_wifi_ctl, wifi_if) = esp_radio::wifi::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(sta_conf),
    )
    .expect("Failed to initialize Wi-Fi controller");
    //espnow
    let espnow = wifi_if.esp_now;
    espnow.set_channel(2).unwrap();
    //led
    let led = Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default());
    spawner.spawn(esp_listener(espnow, led).unwrap());
    println!("listening..");
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
#[embassy_executor::task]
async fn esp_listener(mut espnow: esp_radio::esp_now::EspNow<'static>, mut led: Output<'static>) {
    loop {
        let packet = espnow.receive_async().await;
        let s = core::str::from_utf8(&packet.data()).unwrap_or("Invalid UTF8");
        if s.starts_with("TOGGLE") {
            println!("recieved a toggle packet");
            led.toggle();
        } else {
            println!("invalid packet");
        }
        Timer::after(Duration::from_millis(10)).await;
    }
}
