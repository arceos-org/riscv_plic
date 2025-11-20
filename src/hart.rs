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

/// The interrupt mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Machine mode
    Machine = 0,
    /// Supervisor mode
    Supervisor = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimpleContext<'a> {
    pub privileges: &'a [u8],
    pub hart_id: usize,
    pub mode: Mode,
}

impl<'a> HartContext for SimpleContext<'a> {
    fn index(self) -> usize {
        assert!(self.mode as u8 <= self.privileges[self.hart_id]);
        self.privileges.iter().take(self.hart_id).sum::<u8>() as usize + self.mode as usize
    }
}
