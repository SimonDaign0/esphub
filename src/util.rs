use esp_hal::gpio::Output;
use esp_radio::esp_now::EspNow;
pub struct Board {
    pub led: Output<'static>,
    pub espnow: EspNow<'static>,
}
use esp_hal::{
    interrupt::software::SoftwareInterruptControl,
    peripherals::{SW_INTERRUPT, TIMG0},
    timer::timg::TimerGroup,
};
#[allow(non_snake_case)]
pub fn start_rtos(SW_INTERRUPT: SW_INTERRUPT<'static>, TIMG0: TIMG0<'static>) {
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 64 * 1024);
    let timg0 = TimerGroup::new(TIMG0);
    let sw_int = SoftwareInterruptControl::new(SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_int.software_interrupt0);
}
