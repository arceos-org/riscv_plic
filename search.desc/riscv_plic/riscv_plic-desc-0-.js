searchState.loadedDescShard("riscv_plic", 0, "RISC-V Platform-Level Interrupt Controller …\nA hart context is a given privilege mode on a given hart.\nTrait for enums of external interrupt source.\nPlatform-Level Interrupt Controller.\nClaim an interrupt in <code>context</code>, returning its source.\nMark that interrupt identified by <code>source</code> is completed in …\nDisable interrupt <code>source</code> in <code>context</code>.\nEnable interrupt <code>source</code> in <code>context</code>.\nReturns the argument unchanged.\nGets priority for interrupt <code>source</code>.\nGet interrupt threshold in <code>context</code>.\nThe identifier number of the interrupt source.\nSee §6.\nInitialize the PLIC by context, setting the priority …\nCalls <code>U::from(self)</code>.\nCheck if interrupt <code>source</code> is enabled in <code>context</code>.\nCheck if interrupt <code>source</code> is pending.\nCreate a new instance of the PLIC from the base address.\nProbe maximum level of priority for interrupt <code>source</code>.\nProbe maximum supported threshold value the <code>context</code> …\nSets priority for interrupt <code>source</code> to <code>value</code>.\nSet interrupt threshold for <code>context</code> to <code>value</code>.")