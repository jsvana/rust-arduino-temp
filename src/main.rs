#![no_std]
#![no_main]

use panic_halt as _;

use arduino_uno::hal::port::mode::Output;
//use arduino_uno::hal::port::portb::{PB2, PB3, PB4, PB5};
use arduino_uno::adc;
use arduino_uno::hal::port::portd::{PD2, PD3, PD4, PD5};
use arduino_uno::prelude::*;

const MIN_TEMP: u16 = 568; // Roughly 40 deg F
const MAX_TEMP: u16 = 625; // Roughly 90 deg F
const LED_COUNT: usize = 8; // 8 LEDs on the board

#[derive(Copy, Clone)]
enum State {
    Low,
    High,
}

fn reset_shift_register(
    shcp: &mut PD2<Output>,
    stcp: &mut PD3<Output>,
    master_reset: &mut PD5<Output>,
) {
    master_reset.set_low().void_unwrap();
    arduino_uno::delay_ms(1);
    master_reset.set_high().void_unwrap();

    stcp.set_low().void_unwrap();
    shcp.set_low().void_unwrap();
}

fn write_to_shift_register(
    shcp: &mut PD2<Output>,
    stcp: &mut PD3<Output>,
    data: &mut PD4<Output>,
    item: &State,
) {
    match item {
        State::High => data.set_high().void_unwrap(),
        State::Low => data.set_low().void_unwrap(),
    }
    shcp.set_high().void_unwrap();
    shcp.set_low().void_unwrap();
    stcp.set_high().void_unwrap();
    stcp.set_low().void_unwrap();
}

fn write_all_to_shift_register(
    shcp: &mut PD2<Output>,
    stcp: &mut PD3<Output>,
    data: &mut PD4<Output>,
    line: [State; LED_COUNT],
) {
    for item in line.iter().rev() {
        write_to_shift_register(shcp, stcp, data, item);
    }
}

fn get_led_count(mut reading: u16) -> u8 {
    let mut count = 0;

    reading -= MIN_TEMP;

    let led_range = (MAX_TEMP - MIN_TEMP) / LED_COUNT as u16;

    loop {
        if reading < led_range {
            if reading > led_range / 2 {
                count += 1;
            }

            break;
        }

        reading -= led_range;
        count += 1;
    }

    count
}

#[arduino_uno::entry]
fn main() -> ! {
    let peripherals = arduino_uno::Peripherals::take().unwrap();

    let mut pins = arduino_uno::Pins::new(peripherals.PORTB, peripherals.PORTC, peripherals.PORTD);

    let mut serial = arduino_uno::Serial::new(
        peripherals.USART0,
        pins.d0,
        pins.d1.into_output(&mut pins.ddr),
        9600.into_baudrate(),
    );

    let mut shcp = pins.d2.into_output(&mut pins.ddr);
    let mut stcp = pins.d3.into_output(&mut pins.ddr);
    let mut data = pins.d4.into_output(&mut pins.ddr);
    let mut master_reset = pins.d5.into_output(&mut pins.ddr);

    reset_shift_register(&mut shcp, &mut stcp, &mut master_reset);

    let mut adc = adc::Adc::new(peripherals.ADC, Default::default());
    let mut a0 = pins.a0.into_analog_input(&mut adc);

    loop {
        let temp_reading: u16 = nb::block!(adc.read(&mut a0)).void_unwrap();

        let clamped_reading = temp_reading.clamp(MIN_TEMP, MAX_TEMP);

        ufmt::uwriteln!(&mut serial, "V(input)  : {}\r", temp_reading).void_unwrap();
        ufmt::uwriteln!(&mut serial, "V(clamped): {}\r", clamped_reading).void_unwrap();

        let set_led_count = get_led_count(clamped_reading);

        let mut line = [State::Low; LED_COUNT];

        for i in 0..set_led_count.into() {
            line[i] = State::High;
        }

        write_all_to_shift_register(&mut shcp, &mut stcp, &mut data, line);

        arduino_uno::delay_ms(1000);
    }
}
