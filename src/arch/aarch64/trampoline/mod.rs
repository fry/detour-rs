use super::meta;
use crate::error::{Error, Result};
use crate::pic;
use bad64::{Imm, Instruction, Op, Operand, Reg};
use dynasmrt::{dynasm, DynasmApi, DynasmLabelApi};
use generic_array::{typenum, ArrayLength, GenericArray};
use std::mem;
use std::ops::{Deref, DerefMut};

/// A trampoline generator (x86/x64).
pub struct Trampoline {
  emitter: pic::CodeEmitter,
  prolog_size: usize,
}

impl Trampoline {
  /// Constructs a new trampoline for an address.
  pub unsafe fn new(target: *const (), margin: usize) -> Result<Trampoline> {
    Builder::new(target, margin).build()
  }

  /// Returns a reference to the trampoline's code emitter.
  pub fn emitter(&self) -> &pic::CodeEmitter {
    &self.emitter
  }

  /// Returns the size of the prolog (i.e the amount of disassembled bytes).
  pub fn prolog_size(&self) -> usize {
    self.prolog_size
  }
}

/// A trampoline builder.
struct Builder {
  // /// Disassembler for x86/x64.
  // disassembler: Disassembler,
  // /// Target destination for a potential internal branch.
  // branch_address: Option<usize>,
  /// Total amount of bytes disassembled.
  total_bytes_disassembled: usize,
  /// The preferred minimum amount of bytes disassembled.
  margin: usize,
  /// Whether disassembling has finished or not.
  finished: bool,
  /// The target the trampoline is adapted for.
  target: *const (),
}

macro_rules! thunk_dynasm {
  ($($t:tt)*) => {{
    let mut ops = dynasmrt::aarch64::Assembler::new().unwrap();
    dynasm!(ops
      ; .arch aarch64
      $($t)*
    );
    let buf = ops.finalize().unwrap();
    buf.deref().to_vec()
  }}
}

impl Builder {
  /// Returns a trampoline builder.
  pub fn new(target: *const (), margin: usize) -> Self {
    Builder {
      // disassembler: Disassembler::new(target),
      // branch_address: None,
      total_bytes_disassembled: 0,
      finished: false,
      target,
      margin,
    }
  }

  /// Creates a trampoline with the supplied settings.
  ///
  /// Margins larger than five bytes may lead to undefined behavior.
  pub fn build(mut self) -> Result<Trampoline> {
    let mem = unsafe { std::slice::from_raw_parts(self.target as *const u8, self.margin) };
    let mut instructions = bad64::disasm(mem, self.target as u64);

    let mut emitter = pic::CodeEmitter::new();

    let mut bytes_disassembled = 0;
    while !self.finished {
      let instruction = instructions
        .next()
        .and_then(|r| r.ok())
        .ok_or(Error::InvalidCode)?;
      bytes_disassembled += 4;

      dbg!(&instruction);

      let thunk = self.process_instruction(&instruction)?;
      emitter.add_thunk(thunk);

      // Copied enough bytes for the prolog, append a branch to the rest of the
      // function
      if bytes_disassembled >= self.margin && !self.finished {
        self.finished = true;
        emitter.add_thunk(self.gen_jmp_immediate(self.target as usize))
      }
    }

    Ok(Trampoline {
      emitter,
      prolog_size: bytes_disassembled,
    })
  }

  fn process_instruction(&mut self, instruction: &Instruction) -> Result<Box<dyn pic::Thunkable>> {
    Ok(match instruction.op() {
      // Instruction relative load instructions
      Op::LDR if matches!(instruction.operands().get(1), Some(Operand::Label(imm))) => {
        unimplemented!()
      },
      Op::ADR => self.copy_adr(instruction)?,
      Op::ADRP => unimplemented!(),
      // Branching instructions
      op if meta::CONDITIONAL_OPS.contains(&op) => unimplemented!(),
      Op::B | Op::BL => unimplemented!(),
      Op::CBZ | Op::CBNZ => unimplemented!(),
      Op::TBZ | Op::TBNZ => unimplemented!(),
      // Plainly copy all other instructions
      _ => Box::new(instruction.opcode().to_le_bytes().to_vec()),
    })
  }

  fn copy_adr(&mut self, instruction: &Instruction) -> Result<Box<dyn pic::Thunkable>> {
    Ok(match instruction.operands() {
      [Operand::Reg { reg, arrspec: None }, Operand::Label(Imm::Unsigned(target))]
        if instruction.op() == Op::ADR =>
      {
        let target = *target as usize;
        let reg = *reg;
        Box::new(pic::FixedThunk::<typenum::U8>::new(move |dest| {
          let delta = target as isize - dest as isize;
          let delta_page = (target / meta::PAGE_SIZE) as isize - (dest / meta::PAGE_SIZE) as isize;

          let max_range = 1isize << 20;
          if delta >= -max_range && delta < max_range {
            GenericArray::clone_from_slice(&thunk_dynasm!(
                ; adr X(reg_no(reg).unwrap()), delta
                ; nop
            ))
          } else if delta_page >= -max_range && delta_page < max_range {
            GenericArray::clone_from_slice(&thunk_dynasm!(
                ; adrp X(reg_no(reg).unwrap()), delta_page
                ; add X(reg_no(reg).unwrap()), X(reg_no(reg).unwrap()), (target & 0xFFFF) as u32
            ))
          } else {
            unimplemented!()
          }
        }))
      }
      _ => unimplemented!(),
    })
  }

  fn gen_jmp_immediate(&mut self, target: usize) -> Box<dyn pic::Thunkable> {
    // generate a branch to an absolute address
    Box::new(thunk_dynasm!(
        ; ldr x17, >target
        ; br x17
        ; target:
        ; .dword target as _
    ))
  }
}

fn reg_no(reg: Reg) -> Option<u32> {
  Some(match reg {
    Reg::X0 => 0,
    Reg::X1 => 1,
    Reg::X2 => 2,
    Reg::X3 => 3,
    Reg::X4 => 4,
    Reg::X5 => 5,
    Reg::X6 => 6,
    Reg::X7 => 7,
    Reg::X8 => 8,
    Reg::X9 => 9,
    Reg::X10 => 10,
    Reg::X11 => 11,
    Reg::X12 => 12,
    Reg::X13 => 13,
    Reg::X14 => 14,
    Reg::X15 => 15,
    Reg::X16 => 16,
    Reg::X17 => 17,
    Reg::X18 => 18,
    Reg::X19 => 19,
    Reg::X20 => 20,
    Reg::X21 => 21,
    Reg::X22 => 22,
    Reg::X23 => 23,
    Reg::X24 => 24,
    Reg::X25 => 25,
    Reg::X26 => 26,
    Reg::X27 => 27,
    Reg::X28 => 28,
    Reg::X29 => 29,
    Reg::X30 => 30,
    _ => return None,
  })
}
