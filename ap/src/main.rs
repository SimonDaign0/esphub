#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
//Embassy
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
// Custom lib
use esphub as lib;
//Esp
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    rng::Rng,
};
extern crate alloc;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    //init configs
    let cl_conf = esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
    let peripherals = esp_hal::init(cl_conf);
    lib::util::start_rtos(peripherals.SW_INTERRUPT, peripherals.TIMG0);
    let rng = Rng::new();
    //
    let (stack, espnow) = lib::gateway::start_wifi(peripherals.WIFI, rng, &spawner).await;

    let led = Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default());
    let board = lib::util::Board { led, espnow };

    spawner.spawn(lib::web::handle_requests(stack, board).unwrap());
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
