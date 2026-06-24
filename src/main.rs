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
use esp_hal::{clock::CpuClock, rng::Rng, timer::timg::TimerGroup};

extern crate alloc;

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    //init configs
    let cl_conf = esp_hal::Config::default().with_cpu_clock(CpuClock::_80MHz);
    let peripherals = esp_hal::init(cl_conf);
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_int =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);
    let rng = Rng::new();
    let stack = lib::gateway::start_wifi(peripherals.WIFI, rng, &spawner).await;
    spawner.spawn(lib::web::handle_requests(stack).expect("Wifi stack handling task error"));
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}
