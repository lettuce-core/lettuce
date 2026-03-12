use crate::cpu;
use core::sync::atomic::{AtomicU64, Ordering};

const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_ICW1_INIT: u8 = 0x10;
const PIC_ICW1_ICW4: u8 = 0x01;

const PIC_ICW3_MASTER: u8 = 4; // slave PIC connected to IRQ2
const PIC_ICW3_SLAVE: u8 = 2; // slave cascade identity

const PIC_ICW4_8086: u8 = 0x01;
const PIC_EOI: u8 = 0x20;

const PIT_CH0: u16 = 0x40;
const PIT_CMD: u16 = 0x43;
const PIT_MODE_RATE_GEN: u8  = 0x34;
const PIT_BASE_HZ: u32 = 1_193_182;
const PIT_BOOT_HZ: u32 = 100;

pub const IRQ_BASE_VECTOR: u8 = 32;
pub const IRQ_SLAVE_VECTOR: u8 = IRQ_BASE_VECTOR + 8;
pub const IRQ_TIMER: u8 = 0;

unsafe extern "C" {
    fn irq0_stub();
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptInitReport {
    pub pic1_offset: u8,
    pub pic2_offset: u8,
    pub timer_hz: u32,
}

impl InterruptInitReport {
    pub fn label(self) -> &'static str {
        "interrupts: pic remapped, pit configured, irq0 active"
    }
}

static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn _ticks() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

pub fn init() -> InterruptInitReport {
    cpu::install_interrupt_gate(irq_vector(IRQ_TIMER), irq0_stub);

    pic_remap(IRQ_BASE_VECTOR, IRQ_SLAVE_VECTOR);
    mask_all_irqs();
    pit_set_frequency(PIT_BOOT_HZ);
    set_irq_mask(IRQ_TIMER, false);

    InterruptInitReport {
        pic1_offset: IRQ_BASE_VECTOR,
        pic2_offset: IRQ_SLAVE_VECTOR,
        timer_hz: PIT_BOOT_HZ,
    }
}

pub fn mask_all_irqs() {
    write_port(PIC1_DATA, 0xFF);
    write_port(PIC2_DATA, 0xFF);
}

pub fn set_irq_mask(irq: u8, masked: bool) {
    let (port, line) = match irq {
        0..=7 => (PIC1_DATA, irq),
        8..=15 => (PIC2_DATA, irq - 8),
        _ => return,
    };

    let bit = 1u8 << line;
    let mut mask = read_port(port);

    if masked { mask |= bit }
    else { mask &= !bit }

    write_port(port, mask);
}

pub fn acknowledge_irq(irq: u8) {
    if irq >= 8 {
        write_port(PIC2_CMD, PIC_EOI);
    }
    
    write_port(PIC1_CMD, PIC_EOI);
}

#[no_mangle]
pub extern "C" fn irq0_entry_rust() {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);
    acknowledge_irq(IRQ_TIMER);
}

fn pic_remap(pic1_offset: u8, pic2_offset: u8) {
    write_port(PIC1_CMD, PIC_ICW1_INIT | PIC_ICW1_ICW4);
    io_wait();
    write_port(PIC2_CMD, PIC_ICW1_INIT | PIC_ICW1_ICW4);
    io_wait();

    write_port(PIC1_DATA, pic1_offset);
    io_wait();
    write_port(PIC2_DATA, pic2_offset);
    io_wait();

    write_port(PIC1_DATA, PIC_ICW3_MASTER);
    io_wait();
    write_port(PIC2_DATA, PIC_ICW3_SLAVE);
    io_wait();

    write_port(PIC1_DATA, PIC_ICW4_8086);
    io_wait();
    write_port(PIC2_DATA, PIC_ICW4_8086);
    io_wait();
}

fn pit_set_frequency(hz: u32) {
    let divisor = ((PIT_BASE_HZ + hz / 2) / hz.max(1))
        .clamp(1, u16::MAX as u32) as u16;

    write_port(PIT_CMD, PIT_MODE_RATE_GEN);
    write_port(PIT_CH0, (divisor & 0x00FF) as u8);
    write_port(PIT_CH0, (divisor >> 8) as u8);
}

fn irq_vector(irq: u8) -> u8 {
    IRQ_BASE_VECTOR + irq
}

fn io_wait() {
    write_port(0x80, 0);
}

#[cfg(target_arch = "x86_64")]
fn write_port(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        )
    }
}

#[cfg(not(target_arch = "x86_64"))]
fn write_port(_port: u16, _value: u8) {}

#[cfg(target_arch = "x86_64")]
fn read_port(port: u16) -> u8 {
    let mut v: u8;
    
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") v,
            options(nomem, nostack, preserves_flags)
        )
    }
    
    v
}

#[cfg(not(target_arch = "x86_64"))]
fn read_port(_port: u16) -> u8 {
    0xFF
}
