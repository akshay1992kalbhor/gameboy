#![allow(dead_code)]

use alu;
use instr::{Arith, HasDuration, Instr, InstrPointer, Jump, Ld, Rotate};
use mem::{Addr, Direction, Memory};
use register::{Flags, Registers, R16, R8};
use register_kind::{RegisterKind16, RegisterKind8};

pub struct Cpu {
    pub registers: Registers,
    pub memory: Memory,
    pub ip: InstrPointer,
}

enum BranchAction {
    Take,
    Skip,
}

impl Cpu {
    pub fn create() -> Cpu {
        Cpu {
            registers: Registers::create(),
            memory: Memory::create(),
            ip: InstrPointer::create(),
        }
    }

    fn indirect_ld(&mut self, k: RegisterKind16) -> (u8, Addr) {
        let r = self.registers.read16(k);
        let addr = Addr::indirectly(r);
        (self.memory.ld8(addr), addr)
    }

    fn indirect_st(&mut self, k: RegisterKind16, n: u8) {
        let r = self.registers.read16(k);
        self.memory.st8(Addr::indirectly(r), n)
    }

    fn execute_ld(&mut self, ld: Ld) -> BranchAction {
        use self::Ld::*;

        match ld {
            RGetsR(r1, r2) => {
                let n = self.registers.read8(r2);
                self.registers.write8r(r1, n);
            }
            RGetsN(r, n) => self.registers.write8n(r, n),
            RGetsHlInd(r) => {
                let (n, _) = self.indirect_ld(RegisterKind16::Hl);
                self.registers.write8n(r, n);
            }
            HlIndGetsR(r) => {
                let n = self.registers.read8(r);
                self.indirect_st(RegisterKind16::Hl, n.0)
            }
            HlIndGetsN(n) => self.indirect_st(RegisterKind16::Hl, n),
            AGetsBcInd => {
                let (n, _) = self.indirect_ld(RegisterKind16::Bc);
                self.registers.write8n(RegisterKind8::A, n);
            }
            AGetsDeInd => {
                let (n, _) = self.indirect_ld(RegisterKind16::De);
                self.registers.write8n(RegisterKind8::A, n);
            }
            AGetsNnInd(nn) => {
                let n = self.memory.ld8(nn);
                self.registers.write8n(RegisterKind8::A, n);
            }
            BcIndGetsA => {
                let n = self.registers.a;
                self.indirect_st(RegisterKind16::Bc, n.0)
            }
            DeIndGetsA => {
                let n = self.registers.a;
                self.indirect_st(RegisterKind16::De, n.0)
            }
            NnIndGetsA(nn) => {
                let n = self.registers.a;
                self.memory.st8(nn, n.0);
            }
            AGetsIOOffset(offset) => {
                let addr = Addr::io_memory().offset(u16::from(offset), Direction::Pos);
                let n = self.memory.ld8(addr);
                self.registers.write8n(RegisterKind8::A, n)
            }
            IOOffsetGetsA(offset) => {
                let addr = Addr::io_memory().offset(u16::from(offset), Direction::Pos);
                let n = self.registers.read8(RegisterKind8::A);
                self.memory.st8(addr, n.0);
            }
            AGetsIOOffsetByC => {
                let offset = self.registers.c();
                let addr = Addr::io_memory().offset(u16::from(offset.0), Direction::Pos);
                let n = self.memory.ld8(addr);
                self.registers.write8n(RegisterKind8::A, n)
            }
            IOOffsetByCGetsA => {
                let offset = self.registers.c();
                let addr = Addr::io_memory().offset(u16::from(offset.0), Direction::Pos);
                let n = self.registers.read8(RegisterKind8::A);
                self.memory.st8(addr, n.0);
            }
            HlIndGetsAInc => {
                let n = self.registers.read8(RegisterKind8::A);
                self.indirect_st(RegisterKind16::Hl, n.0);
                self.registers.hl.inc()
            }
            AGetsHlIndInc => {
                let (n, _) = self.indirect_ld(RegisterKind16::Hl);
                self.registers.write8n(RegisterKind8::A, n);
                self.registers.hl.inc()
            }
            HlIndGetsADec => {
                let n = self.registers.read8(RegisterKind8::A);
                self.indirect_st(RegisterKind16::Hl, n.0);
                self.registers.hl.dec()
            }
            AGetsHlIndDec => {
                let (n, _) = self.indirect_ld(RegisterKind16::Hl);
                self.registers.write8n(RegisterKind8::A, n);
                self.registers.hl.dec()
            }
            SpGetsAddr(addr) => {
                self.registers.sp = addr.into_register();
            }
            HlGetsAddr(addr) => {
                self.registers.hl = addr.into_register();
            }
            DeGetsAddr(addr) => {
                self.registers.de = addr.into_register();
            }
        };
        BranchAction::Take
    }

    fn execute_alu_binop<F>(&mut self, f: F, operand: u8) -> u8
    where
        F: FnOnce(&mut Flags, u8, u8) -> u8,
    {
        let old_a = self.registers.a.0;
        let result = f(&mut self.registers.flags, old_a, operand);
        self.registers.write8n(RegisterKind8::A, result);
        result
    }

    fn execute_arith(&mut self, arith: Arith) -> BranchAction {
        use self::Arith::*;

        match arith {
            Xor(r) => {
                let operand = self.registers.read8(r).0;
                self.execute_alu_binop(alu::xor, operand);
            }
            XorHlInd => {
                let (operand, _) = self.indirect_ld(RegisterKind16::Hl);
                self.execute_alu_binop(alu::xor, operand);
            }
            Sub(r) => {
                let operand = self.registers.read8(r).0;
                self.execute_alu_binop(alu::sub, operand);
            }
            SubHlInd => {
                let (operand, _) = self.indirect_ld(RegisterKind16::Hl);
                self.execute_alu_binop(alu::sub, operand);
            }
            AddHlInd => {
                let (operand, _) = self.indirect_ld(RegisterKind16::Hl);
                self.execute_alu_binop(alu::add, operand);
            }
            AddN(n) => {
                self.execute_alu_binop(alu::add, n);
            }
            Inc8(r) => {
                let operand = self.registers.read8(r);
                let result = alu::inc(&mut self.registers.flags, operand.0);
                self.registers.write8n(r, result);
            }
            Inc16(r16) => {
                let operand = self.registers.read16(r16);
                let result = alu::inc16(&mut self.registers.flags, operand.0);
                self.registers.write16n(r16, result);
            }
            IncHlInd => {
                let (operand, addr) = self.indirect_ld(RegisterKind16::Hl);
                let result = alu::inc(&mut self.registers.flags, operand);
                self.memory.st8(addr, result);
            }
            Dec8(r) => {
                let operand = self.registers.read8(r);
                let result = alu::dec(&mut self.registers.flags, operand.0);
                self.registers.write8n(r, result);
            }
            Dec16(r16) => {
                let operand = self.registers.read16(r16);
                let result = alu::dec16(&mut self.registers.flags, operand.0);
                self.registers.write16n(r16, result);
            }
            DecHlInd => {
                let (operand, addr) = self.indirect_ld(RegisterKind16::Hl);
                let result = alu::dec(&mut self.registers.flags, operand);
                self.memory.st8(addr, result);
            }
        };
        BranchAction::Take
    }

    fn execute_rotate(&mut self, rotate: Rotate) -> BranchAction {
        use self::Rotate::*;

        match rotate {
            Rla => {
                let n = self.registers.read8(RegisterKind8::A);
                let result = alu::rl(&mut self.registers.flags, n.0);
                self.registers.write8n(RegisterKind8::A, result);
            }
            Rl(r) => {
                let n = self.registers.read8(r);
                let result = alu::rl(&mut self.registers.flags, n.0);
                self.registers.write8n(r, result);
            }
        };
        BranchAction::Take
    }

    fn pop(&mut self) -> R8 {
        self.registers.sp.inc();
        let (v, _) = self.indirect_ld(RegisterKind16::Sp);
        R8(v)
    }

    fn pop16(&mut self) -> R16 {
        let hi = self.pop();
        let lo = self.pop();
        hi.concat(lo)
    }

    fn push(&mut self, n: R8) {
        self.indirect_st(RegisterKind16::Sp, n.0);
        self.registers.sp.dec();
    }

    fn push16(&mut self, n: R16) {
        self.push(n.lo());
        self.push(n.hi());
    }
}

#[cfg(test)]
mod instr_tests {
    use cpu::Cpu;
    use register::R16;
    use test::proptest::prelude::*;

    proptest! {
        #[test]
        fn push_pop_self_inverse(x : u16) {
            let mut cpu = Cpu::create();
            cpu.registers.sp = R16(0xff90);

            let r16 = R16(x);
            cpu.push16(r16);
            let res = cpu.pop16();
            assert_eq!(res, r16);
        }
    }
}

impl Cpu {
    fn do_call(&mut self, addr: Addr) {
        let r16 = self.ip.0.into_register();
        self.push16(r16);
        self.ip.jump(addr);
    }

    fn execute_jump(&mut self, jump: Jump) -> BranchAction {
        use self::Jump::*;

        match jump {
            Jr(offset) => {
                self.ip.offset_by(offset);
                BranchAction::Take
            }
            JrNz(offset) => {
                if !self.registers.flags.z {
                    self.ip.offset_by(offset);
                    BranchAction::Take
                } else {
                    BranchAction::Skip
                }
            }
            JrZ(offset) => {
                if self.registers.flags.z {
                    self.ip.offset_by(offset);
                    BranchAction::Take
                } else {
                    BranchAction::Skip
                }
            }
            Call(addr) => {
                self.do_call(addr);
                BranchAction::Take
            }
            CallZ(addr) => {
                if self.registers.flags.z {
                    self.do_call(addr);
                    BranchAction::Take
                } else {
                    BranchAction::Skip
                }
            }
        }
    }

    fn execute_instr(&mut self, instr: Instr) -> BranchAction {
        use self::Instr::*;

        match instr {
            Nop => BranchAction::Take,
            Ld(ld) => self.execute_ld(ld),
            Arith(arith) => self.execute_arith(arith),
            Rotate(rotate) => self.execute_rotate(rotate),
            Jump(jump) => self.execute_jump(jump),
            Bit7h => {
                let x = self.registers.read8(RegisterKind8::H);
                alu::bit(&mut self.registers.flags, x.0, 7);
                BranchAction::Take
            }
            CpHlInd => {
                let x = self.registers.read8(RegisterKind8::A);
                let (operand, _) = self.indirect_ld(RegisterKind16::Hl);
                let _ = alu::sub(&mut self.registers.flags, x.0, operand);
                BranchAction::Take
            }
            Cp(n) => {
                let x = self.registers.read8(RegisterKind8::A);
                let _ = alu::sub(&mut self.registers.flags, x.0, n);
                BranchAction::Take
            }
            PopBc => {
                // TODO: Are we pushing and popping the stack in the right order
                self.registers.bc = self.pop16();
                BranchAction::Take
            }
            PushBc => {
                let bc = self.registers.bc;
                self.push16(bc);
                BranchAction::Take
            }
            Ret => {
                let reg = self.pop16();
                self.ip.jump(Addr::directly(reg.0));
                BranchAction::Take
            }
        }
    }

    /// Peek at the next instruction to see how long it will take
    pub fn peek_next(&mut self) -> u32 {
        let instr = self.ip.peek(&self.memory);
        // Assumption: Take duration is longer than skip duration (this seems to be true for all
        // Gameboy instructions
        let (take_duration, _) = instr.duration();
        take_duration
    }

    /// Execute the current instruction returning the duration it did take. Note: This can be
    /// different from the peeked duration as time taken differs based on branch takes or skips.
    pub fn execute(&mut self) -> u32 {
        let instr = self.ip.read(&self.memory);
        let (take_duration, skip_duration) = instr.duration();
        let action = self.execute_instr(instr);
        match action {
            BranchAction::Take => take_duration,
            BranchAction::Skip => skip_duration.unwrap_or_else(|| take_duration),
        }
    }
}
