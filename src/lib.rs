//! RISC-V Platform-Level Interrupt Controller
//! https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc
#![no_std]
#![feature(const_option)]
#![feature(const_nonnull_new)]

use core::num::NonZeroU32;
use core::ptr::NonNull;

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_structs,
    registers::{ReadOnly, ReadWrite},
};

/// See §1.
const SOURCE_NUM: usize = 1024;
/// See §1.
const CONTEXT_NUM: usize = 15872;

const U32_BITS: usize = u32::BITS as usize;

register_structs! {
  #[allow(non_snake_case)]
  ContextLocal {
    /// Priority Threshold
    /// - The base address of Priority Thresholds register block is located at 4K alignment starts from offset 0x200000.
    (0x0000 => PriorityThreshold: ReadWrite<u32>),
    /// Interrupt Claim/complete Process
    /// - The Interrupt Claim Process register is context based and is located at (4K alignment + 4) starts from offset 0x200000.
    (0x0004 => InterruptClaimComplete: ReadWrite<u32>),
    (0x0008 => _reserved_0),
    (0x1000 => @END),
  }
}

register_structs! {
  #[allow(non_snake_case)]
  InterruptEnableCtxX {
    /// Priority Threshold
    /// - The base address of Priority Thresholds register block is located at 4K alignment starts from offset 0x200000.
    (0x00 => InterruptSources: [ReadWrite<u32>; SOURCE_NUM / U32_BITS]),
    (0x80 => @END),
  }
}

register_structs! {
  #[allow(non_snake_case)]
  PLICRegs {
    /// Interrupt Source Priority #0 to #1023
    (0x000000 => InterruptPriority: [ReadWrite<u32>; SOURCE_NUM]),
    /// Interrupt Pending Bit of Interrupt Source #0 to #N
    /// 0x001000: Interrupt Source #0 to #31 Pending Bits
    /// ...
    /// 0x00107C: Interrupt Source #992 to #1023 Pending Bits
    (0x001000 => InterruptPending: [ReadOnly<u32>; 0x20]),
    (0x001080 => _reserved_0),
    /// Interrupt Enable Bit of Interrupt Source #0 to #1023 for 15872 contexts
    (0x002000 => InterruptEnableCtxX: [InterruptEnableCtxX; CONTEXT_NUM]),
    (0x1F2000 => _reserved_1),
    /// 4096 * 15872 = 65011712(0x3e000 00) bytes
    /// Priority Threshold for 15872 contexts
    /// - The base address of Priority Thresholds register block is located at 4K alignment starts from offset 0x200000.
    /// Interrupt Claim Process for 15872 contexts
    /// - The Interrupt Claim Process register is context based and is located at (4K alignment + 4) starts from offset 0x200000.
    /// - The Interrupt Completion registers are context based and located at the same address with Interrupt Claim Process register, which is at (4K alignment + 4) starts from offset 0x200000.
    (0x200000 => Contexts: [ContextLocal; CONTEXT_NUM]),
    (0x4000000 => @END),
  }
}

/// Trait for enums of external interrupt source.
///
/// See §1.4.
pub trait InterruptSource {
    /// The identifier number of the interrupt source.
    fn id(self) -> NonZeroU32;
}

/// A hart context is a given privilege mode on a given hart.
///
/// See §1.1.
pub trait HartContext {
    /// See §6.
    ///
    /// > How PLIC organizes interrupts for the contexts (Hart and privilege mode)
    /// > is out of RISC-V PLIC specification scope, however it must be spec-out
    /// > in vendor’s PLIC specification.
    fn index(self) -> usize;
}

pub struct Plic {
    base: NonNull<PLICRegs>,
}

unsafe impl Send for Plic {}
unsafe impl Sync for Plic {}

impl Plic {
    /// Create a new instance of the PLIC from the base address.
    pub const fn new(base: *mut u8) -> Self {
        Self {
            base: NonNull::new(base).unwrap().cast(),
        }
    }

    /// Initialize the PLIC by context, setting the priority threshold to 0.
    pub fn init_by_context<C>(&mut self, context: C)
    where
        C: HartContext,
    {
        self.regs().Contexts[context.index()]
            .PriorityThreshold
            .set(0);
    }

    const fn regs(&self) -> &PLICRegs {
        unsafe { self.base.as_ref() }
    }

    /// Sets priority for interrupt `source` to `value`.
    ///
    /// Write `0` to priority `value` effectively disables this interrupt `source`, for the priority
    /// value 0 is reserved for "never interrupt" by the PLIC specification.
    ///
    /// The lowest active priority is priority `1`. The maximum priority depends on PLIC implementation
    /// and can be detected with [`Plic::probe_priority_bits`].
    ///
    /// See §4.
    #[inline]
    pub fn set_priority<S>(&self, source: S, value: u32)
    where
        S: InterruptSource,
    {
        self.regs().InterruptPriority[source.id().get() as usize].set(value);
    }

    /// Gets priority for interrupt `source`.
    ///
    /// See §4.
    #[inline]
    pub fn get_priority<S>(&self, source: S) -> u32
    where
        S: InterruptSource,
    {
        self.regs().InterruptPriority[source.id().get() as usize].get()
    }

    /// Probe maximum level of priority for interrupt `source`.
    ///
    /// See §4.
    #[inline]
    pub fn probe_priority_bits<S>(&self, source: S) -> u32
    where
        S: InterruptSource,
    {
        let source = source.id().get() as usize;
        self.regs().InterruptPriority[source].set(!0);
        self.regs().InterruptPriority[source].get()
    }

    /// Check if interrupt `source` is pending.
    ///
    /// See §5.
    #[inline]
    pub fn is_pending<S>(&self, source: S) -> bool
    where
        S: InterruptSource,
    {
        let (group, index) = parse_group_and_index(source.id().get() as usize);
        self.regs().InterruptPending[group].get() & (1 << index) != 0
    }

    /// Enable interrupt `source` in `context`.
    ///
    /// See §6.
    #[inline]
    pub fn enable<S, C>(&self, source: S, context: C)
    where
        S: InterruptSource,
        C: HartContext,
    {
        let context = context.index();
        let (group, index) = parse_group_and_index(source.id().get() as usize);

        let value = self.regs().InterruptEnableCtxX[context].InterruptSources[group].get();
        self.regs().InterruptEnableCtxX[context].InterruptSources[group].set(value | 1 << index);
    }

    /// Disable interrupt `source` in `context`.
    ///
    /// See §6.
    #[inline]
    pub fn disable<S, C>(&self, source: S, context: C)
    where
        S: InterruptSource,
        C: HartContext,
    {
        let context = context.index();
        let (group, index) = parse_group_and_index(source.id().get() as usize);

        let value = self.regs().InterruptEnableCtxX[context].InterruptSources[group].get();
        self.regs().InterruptEnableCtxX[context].InterruptSources[group].set(value & !(1 << index));
    }

    /// Check if interrupt `source` is enabled in `context`.
    ///
    /// See §6.
    #[inline]
    pub fn is_enabled<S, C>(&self, source: S, context: C) -> bool
    where
        S: InterruptSource,
        C: HartContext,
    {
        let context = context.index();
        let (group, index) = parse_group_and_index(source.id().get() as usize);

        self.regs().InterruptEnableCtxX[context].InterruptSources[group].get() & (1 << index) != 0
    }

    /// Get interrupt threshold in `context`.
    ///
    /// See §7.
    #[inline]
    pub fn get_threshold<C>(&self, context: C) -> u32
    where
        C: HartContext,
    {
        self.regs().Contexts[context.index()]
            .PriorityThreshold
            .get()
    }

    /// Set interrupt threshold for `context` to `value`.
    ///
    /// See §7.
    #[inline]
    pub fn set_threshold<C>(&self, context: C, value: u32)
    where
        C: HartContext,
    {
        self.regs().Contexts[context.index()]
            .PriorityThreshold
            .set(value);
    }

    /// Probe maximum supported threshold value the `context` supports.
    ///
    /// See §7.
    #[inline]
    pub fn probe_threshold_bits<C>(&self, context: C) -> u32
    where
        C: HartContext,
    {
        let context = context.index();
        self.regs().Contexts[context].PriorityThreshold.set(!0);
        self.regs().Contexts[context].PriorityThreshold.get()
    }

    /// Claim an interrupt in `context`, returning its source.
    ///
    /// It is always legal for a hart to perform a claim even if `EIP` is not set.
    /// A hart could set threshold to maximum to disable interrupt notification, but it does not mean
    /// interrupt source has stopped to send interrupt signals. In this case, hart would instead
    /// poll for active interrupt by periodically calling the `claim` function.
    ///
    /// See §8.
    #[inline]
    pub fn claim<C>(&self, context: C) -> Option<NonZeroU32>
    where
        C: HartContext,
    {
        NonZeroU32::new(
            self.regs().Contexts[context.index()]
                .InterruptClaimComplete
                .get(),
        )
    }

    /// Mark that interrupt identified by `source` is completed in `context`.
    ///
    /// See §9.
    #[inline]
    pub fn complete<C, S>(&self, context: C, source: S)
    where
        C: HartContext,
        S: InterruptSource,
    {
        self.regs().Contexts[context.index()]
            .InterruptClaimComplete
            .set(source.id().get());
    }
}

fn parse_group_and_index(source: usize) -> (usize, usize) {
    let group = source / U32_BITS;
    let index = source % U32_BITS;
    (group, index)
}
