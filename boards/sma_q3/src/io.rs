use core::fmt::Write;
use core::panic::PanicInfo;
use cortexm4;
use kernel::debug;
use kernel::debug::IoWrite;
use kernel::hil::led;
use nrf52840::gpio::Pin;

use crate::CHIP;
use crate::PROCESSES;
use crate::PROCESS_PRINTER;

enum Writer {
    Uninitialized,
    WriterRtt(&'static extra_capsules::segger_rtt::SeggerRttMemory<'static>),
}

static mut WRITER: Writer = Writer::Uninitialized;

fn wait() {
    for _ in 0..300 {
        cortexm4::support::nop();
    }
}

/// Set the RTT memory buffer used to output panic messages.
pub unsafe fn set_rtt_memory(
    rtt_memory: &'static mut extra_capsules::segger_rtt::SeggerRttMemory<'static>,
) {
    WRITER = Writer::WriterRtt(rtt_memory);
}

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        self.write(s.as_bytes());
        Ok(())
    }
}

impl IoWrite for Writer {
    fn write(&mut self, buf: &[u8]) {
        match self {
            Writer::Uninitialized => {}
            Writer::WriterRtt(rtt_memory) => {
                let up_buffer = unsafe { &*rtt_memory.get_up_buffer_ptr() };
                let buffer_len = up_buffer.length.get();
                let buffer = unsafe {
                    core::slice::from_raw_parts_mut(
                        up_buffer.buffer.get() as *mut u8,
                        buffer_len as usize,
                    )
                };

                let mut write_position = up_buffer.write_position.get();

                for &c in buf {
                    wait();
                    buffer[write_position as usize] = c;
                    write_position = (write_position + 1) % buffer_len;
                    up_buffer.write_position.set(write_position);
                    wait();
                }
            }
        };
    }
}

#[cfg(not(test))]
#[no_mangle]
#[panic_handler]
/// Panic handler
pub unsafe extern "C" fn panic_fmt(pi: &PanicInfo) -> ! {
    // The display LEDs (see back of board)
    let led_kernel_pin = &nrf52840::gpio::GPIOPin::new(Pin::P0_13);
    let led = &mut led::LedLow::new(led_kernel_pin);
    let writer = &mut WRITER;
    debug::panic(
        &mut [led],
        writer,
        pi,
        &cortexm4::support::nop,
        &PROCESSES,
        &CHIP,
        &PROCESS_PRINTER,
    )
}
