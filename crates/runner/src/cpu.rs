use crate::trace::{Access, Tracer};

/// CPU state: 32 general-purpose registers and program counter.
pub struct Cpu {
    /// General-purpose registers x0-x31. x0 is hardwired to 0.
    regs: [u32; 32],
    /// Program counter.
    pub pc: u32,
}

impl Cpu {
    /// Create a new CPU with all registers zeroed apart from sp and gp, and pc at entry point.
    pub fn new(entry_pc: u32, sp: u32, gp: u32) -> Self {
        let mut regs = [0u32; 32];
        regs[2] = sp; // x2 = sp
        regs[3] = gp; // x3 = gp
        Self { regs, pc: entry_pc }
    }

    /// Read register value. x0 always returns 0.
    #[inline]
    pub fn reg(&self, idx: u8) -> u32 {
        if idx == 0 {
            0
        } else {
            self.regs[idx as usize]
        }
    }

    /// Write register value. Writes to x0 are ignored.
    #[inline]
    pub fn set_reg(&mut self, idx: u8, val: u32) {
        if idx != 0 {
            self.regs[idx as usize] = val;
        }
    }

    /// Advance PC by 4 bytes (one instruction).
    #[inline]
    pub fn advance_pc(&mut self) {
        self.pc = self.pc.wrapping_add(4);
    }

    // =========================================================================
    // Traced access methods
    // =========================================================================

    /// Read register with trace tracking. Returns Access with clock info.
    #[inline]
    pub fn read_reg(&self, idx: u8, tracer: &mut Tracer) -> Access {
        let value = self.reg(idx);
        let clk_prev = tracer.reg_clk[idx as usize];
        tracer.reg_clk[idx as usize] = tracer.clk;

        Access {
            addr: idx as u32,
            prev: value,
            clk_prev,
            next: value,
            clk: tracer.clk,
        }
    }

    /// Write register with trace tracking. Returns Access with clock info.
    #[inline]
    pub fn write_reg(&mut self, idx: u8, val: u32, tracer: &mut Tracer) -> Access {
        let prev = self.reg(idx);
        let clk_prev = tracer.reg_clk[idx as usize];
        self.set_reg(idx, val);
        tracer.reg_clk[idx as usize] = tracer.clk;

        Access {
            addr: idx as u32,
            prev,
            clk_prev,
            next: if idx == 0 { 0 } else { val },
            clk: tracer.clk,
        }
    }
}
