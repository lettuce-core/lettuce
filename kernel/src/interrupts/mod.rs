const PIC1_CMD: u16 = 0x20;
const PIC1_DATA: u16 = 0x21;
const PIC2_CMD: u16 = 0xA0;
const PIC2_DATA: u16 = 0xA1;

const PIC_ICW1_INIT: u8 = 0x10;
const PIC_ICW1_ICW4: u8 = 0x01;
const PIC_ICW4_8086: u8 = 0x01;

const PIC_ICW3_MASTER: u8 = 4; // slave PIC connected to IRQ2
const PIC_ICW3_SLAVE: u8 = 2; // slave cascade identity

pub const IRQ_BASE_VECTOR: u8 = 32;
pub const IRQ_SLAVE_BASE_VECTOR: u8 = IRQ_BASE_VECTOR + 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptInitReport {
    pub master_offset: u8,
    pub slave_offset: u8,
}

impl InterruptInitReport {
    pub fn label(self) -> &'static str {
        "interrupts: pic remapped and masked"
    }
}

pub fn init() -> InterruptInitReport {
    pic_remap(IRQ_BASE_VECTOR, IRQ_SLAVE_BASE_VECTOR);
    mask_all_irqs();

    InterruptInitReport {
        master_offset: IRQ_BASE_VECTOR,
        slave_offset: IRQ_SLAVE_BASE_VECTOR,
    }
}

pub fn mask_all_irqs() {
    // :: todo: later the irq dispatch table should decide which lines to unmask
    write_port(PIC1_DATA, 0xFF);
    write_port(PIC2_DATA, 0xFF);
}

fn pic_remap(master_offset: u8, slave_offset: u8) {
    write_port(PIC1_CMD, PIC_ICW1_INIT | PIC_ICW1_ICW4);
    io_wait();
    write_port(PIC2_CMD, PIC_ICW1_INIT | PIC_ICW1_ICW4);
    io_wait();

    write_port(PIC1_DATA, master_offset);
    io_wait();
    write_port(PIC2_DATA, slave_offset);
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
        );
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
        );
    }

    v
}

#[allow(unused)]
#[cfg(not(target_arch = "x86_64"))]
fn read_port(_port: u16) -> u8 {
    0xff
}
