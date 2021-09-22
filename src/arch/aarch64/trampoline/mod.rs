use super::{meta, thunk};
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
    let mem = unsafe { std::slice::from_raw_parts(self.target as *const u8, self.margin + 4) };
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

      let thunk = self.copy_instruction(&instruction)?;
      emitter.add_thunk(thunk);

      // Function ends here
      if self.instruction_ends_code(&instruction) {
        self.finished = true;
      }

      // Copied enough bytes for the prolog, append a branch to the rest of the
      // function
      if bytes_disassembled >= self.margin && !self.finished {
        self.finished = true;
        let next_instruction = instructions
          .next()
          .and_then(|r| r.ok())
          .ok_or(Error::InvalidCode)?;
        emitter.add_thunk(thunk::gen_jmp_immediate(next_instruction.address() as usize));
      }
    }

    Ok(Trampoline {
      emitter,
      prolog_size: bytes_disassembled,
    })
  }

  // Copy the instruction into a position-independant thunk, or one that will
  // generate the correct code for the offset
  fn copy_instruction(&mut self, instruction: &Instruction) -> Result<Box<dyn pic::Thunkable>> {
    Ok(match instruction.op() {
      // Instruction relative load instructions
      Op::LDR if matches!(instruction.operands().get(1), Some(Operand::Label(_))) => {
        thunk::gen_ldr_literal(instruction)?
      },
      Op::ADR => thunk::gen_adr(instruction)?,
      Op::ADRP => thunk::gen_adrp(instruction)?,
      // Branching instructions
      op if meta::CONDITIONAL_OPS.contains(&op) => unimplemented!(),
      Op::B | Op::BL => unimplemented!(),
      Op::CBZ | Op::CBNZ => unimplemented!(),
      Op::TBZ | Op::TBNZ => unimplemented!(),
      // Plainly copy all other instructions
      _ => Box::new(instruction.opcode().to_le_bytes().to_vec()),
    })
  }

  fn instruction_ends_code(&mut self, instruction: &Instruction) -> bool {
    matches!(instruction.op(), Op::RET | Op::B | Op::BR)
  }
}

// struct Displacement {
//   page_delta: isize,
//   addr_delta: isize
// }
// fn get_displacement(dest: usize, target: usize) -> Displacement {
//   Displacement {
//     page_delta:
//     addr_delta: target as isize - dest as isize
//   }
// }
